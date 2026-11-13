//! Heggie's global regularisation, the regularisation occupant of integrator_contract Part 2b § "Heggie: the equations,
//! the time transformations, the maps and the step control (transcribed)" (D. C. Heggie, Celest. Mech. 10 (1974)
//! 217–241; R-160, R-161). Planar, `G = 1`.
//!
//! **Three relative vectors, no reference body, registered once.** `qᵢ = rⱼ − rₖ`, `(j, k)` the cyclic successors of
//! `i`, each Levi-Civita regularised (`Qᵢ`, the conditioned inverse of [`super::az::lc_inverse`]) with its momentum
//! `Pᵢ = 2L(Qᵢ)ᵀpᵢ` ([`register`]); the state `(Q₀, P₀, Q₁, P₁, Q₂, P₂, t)` is marched in fictitious time `τ` under
//! `Γ*` with the energy `h` frozen at registration ([`HgSystem`], its [`Flow`]), and its Cartesian image is
//! [`HgSystem::image`]. [`march`] registers once, at `t = 0`, and carries the state across every boundary of R-565's
//! grid, never re-registering; how the occupant seam carries it across `ADVANCE` calls is RQ-808's, and this module
//! has no [`super::Regularisation`] impl (TASK-M3-75).
//!
//! **The time transformations** ([`HeggieTime`]; R-161): Eq. 20, `dτ = dt/(R₀R₁R₂)`, and Eq. 22 at n = 3/2, the
//! default ([`DEFAULT_TIME`]), `dτ = S^{3/2}/(R₀R₁R₂) dt`, whose momentum rates gain `3Γ*Qᵢ/S`, `S` floored at `TINY`.
//!
//! **The step control** ([`march`]), the reference driver's as Part 2b transcribes it: the interval `dt_left = Δ_sync`
//! with the interval clock reset at its start; an entry `dt/dτ` that is not positive or not finite ends the march not
//! finite; each step's size `min(η·dt_left/d, dτ_entry)`, `d` the floored `dt/dτ`, cut by the predictive limit
//! `f·d_min/(v_max·dt/dτ)` on the relative velocities of Heggie's Eq. 7, then by the landing clamp; one step of the
//! stepper; the landing secant; the explicit finiteness exit and R-604's budget. The floor signal (`dt/dτ < TINY`,
//! `S < TINY` under Eq. 22, or any `Rᵢ < TINY`) and the entry-cap hit (`η·dt_left/d > dτ_entry`) are counted,
//! non-terminal signals (Part 6, R-563). Each step's physical increment is fed to `dt_max` ([`super::dt_max`]).
//!
//! **The regularised energy** ([`HgSystem::energy`]; integrator_contract Part 1, R-566, REQ-INT-104): Heggie's Eq. 6
//! on the relative positions and momenta the regularised state gives, read from `(Q, P)`, never through the Cartesian
//! return. **The time-reversal map** ([`reversed`]; Part 2b, REQ-INT-095): `(Q, P, t) → (Q, −P, −t)`, `h` held.
//!
//! **The stepper slot under Heggie** is [`HgStepper`]: every stepper fills it through its form on the regularised
//! system ([`super::form`]), the pair's profile its form's (Part 2b, R-561).
//!
//! No precision-dependent constant is in a branch here (Part 2c rule 1): `TINY` is the one excepted floor, and the
//! landing tolerance is supplied by the caller ([`HgParams::land_eps_rel`]). What replaces the reference's
//! per-precision `LAND_EPS_REL` in Heggie's landing is RQ-812, open: until it is ruled the landing reads the supplied
//! value as the reference's driver does, and its tests supply the reference's values. The step control's values are
//! calibrations (R-71: REQ-INT-080, REQ-INT-105, REQ-INT-108, REQ-INT-103), read from [`HgParams`], never from a
//! literal here.

use super::az::{
    abs, add, dot, finite, floored, lc, lc_inverse, least, lmul, ltmul, norm, over_allowance,
    scale, sub, two, AzReal, End, ExplicitTest, V,
};
use super::dt_max::DtMax;
use super::form::Flow;
use super::pairs::HeggieTime;
use crate::driver::advance::Cadence;
use crate::occupant::{CapabilityProfile, Geom, State, Stepper};

/// Heggie's default time transformation: Eq. 22 at n = 3/2 (R-161).
pub const DEFAULT_TIME: HeggieTime = HeggieTime::Eq22;

/// The regularised positions and the interval clock: `(Q₀, Q₁, Q₂, t)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HgQ<R> {
    /// `Q₀`, `Q₁`, `Q₂`: `Qᵢ` regularises `qᵢ = rⱼ − rₖ`.
    pub q: [V<R>; 3],
    /// The interval clock `t`, physical time since the interval's boundary.
    pub t: R,
}

/// The regularised momenta `(P₀, P₁, P₂)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HgP<R> {
    /// `Pᵢ = 2L(Qᵢ)ᵀpᵢ`.
    pub p: [V<R>; 3],
}

/// The system: the masses and their factors, the energy `h` frozen at registration, the floor and the time
/// transformation in force. It depends on nothing but the masses and `h`, so it is built once per trajectory.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HgSystem<R> {
    /// The masses `mᵢ`.
    pub m: [R; 3],
    /// `M = m₀ + m₁ + m₂`.
    pub total: R,
    /// `μᵢ = mⱼmₖ/(mⱼ + mₖ)`, the reduced mass of the pair `qᵢ` joins.
    pub mu: [R; 3],
    /// `mmᵢ = mⱼmₖ`.
    pub mm: [R; 3],
    /// `1/mᵢ`.
    pub inv_m: [R; 3],
    /// The energy `h`, frozen at registration.
    pub h: R,
    /// The floor `TINY`.
    pub tiny: R,
    /// The time transformation in force.
    pub time: HeggieTime,
}

/// `Rᵢ = |Qᵢ|²`, unfloored.
#[inline]
fn rr<R: AzReal>(q: &HgQ<R>) -> [R; 3] {
    [
        dot(q.q[0], q.q[0]),
        dot(q.q[1], q.q[1]),
        dot(q.q[2], q.q[2]),
    ]
}

/// `Wᵢ = L(Qᵢ)Pᵢ`.
#[inline]
fn ws<R: AzReal>(q: &HgQ<R>, p: &HgP<R>) -> [V<R>; 3] {
    [
        lmul(q.q[0], p.p[0]),
        lmul(q.q[1], p.p[1]),
        lmul(q.q[2], p.p[2]),
    ]
}

/// The masses' factors of index `i` read cyclically: `(μᵢ, 1/mⱼ, 1/mₖ, mmⱼ, mmₖ)` for `(j, k)` the successors of `i`.
#[derive(Clone, Copy)]
struct Cyc<R> {
    mu_i: R,
    inv_mi: R,
    inv_mj: R,
    inv_mk: R,
    mm_j: R,
    mm_k: R,
}

/// `∂Γ*/∂Pᵢ = Pᵢ RⱼRₖ/(4μᵢ) − [L(Qᵢ)ᵀWₖ·Rⱼ/mⱼ + L(Qᵢ)ᵀWⱼ·Rₖ/mₖ]/4` (Part 2b, Eq. 21).
#[inline]
fn dgamma_dp<R: AzReal>(qi: V<R>, pi: V<R>, wj: V<R>, wk: V<R>, rj: R, rk: R, c: Cyc<R>) -> V<R> {
    let four = two::<R>() * two::<R>();
    sub(
        scale(rj * rk / (four * c.mu_i), pi),
        scale(
            R::one() / four,
            add(
                scale(rj * c.inv_mj, ltmul(qi, wk)),
                scale(rk * c.inv_mk, ltmul(qi, wj)),
            ),
        ),
    )
}

/// The momenta and their regularised counterparts of the other two indices, for [`dgamma_dq`].
#[derive(Clone, Copy)]
struct Others<R> {
    pj: V<R>,
    pk: V<R>,
    wj: V<R>,
    wk: V<R>,
    rj: R,
    rk: R,
    mu_j: R,
    mu_k: R,
}

/// `∂Γ*/∂Qᵢ = Qᵢ(Rₖ|Pⱼ|²/μⱼ + Rⱼ|Pₖ|²/μₖ)/4 − [2Qᵢ Wⱼ·Wₖ/mᵢ + L(Pᵢ)ᵀWₖ·Rⱼ/mⱼ + L(Pᵢ)ᵀWⱼ·Rₖ/mₖ]/4
/// − 2Qᵢ(mmⱼRₖ + mmₖRⱼ) − 2h RⱼRₖ Qᵢ` (Part 2b, Eq. 21).
#[inline]
fn dgamma_dq<R: AzReal>(qi: V<R>, pi: V<R>, o: Others<R>, c: Cyc<R>, h: R) -> V<R> {
    let two = two::<R>();
    let four = two * two;
    let kinetic = scale(
        (o.rk * dot(o.pj, o.pj) / o.mu_j + o.rj * dot(o.pk, o.pk) / o.mu_k) / four,
        qi,
    );
    let coupling = scale(
        R::one() / four,
        add(
            add(
                scale(two * c.inv_mi * dot(o.wj, o.wk), qi),
                scale(o.rj * c.inv_mj, ltmul(pi, o.wk)),
            ),
            scale(o.rk * c.inv_mk, ltmul(pi, o.wj)),
        ),
    );
    let potential = scale(two * (c.mm_j * o.rk + c.mm_k * o.rj), qi);
    let energy = scale(two * h * o.rj * o.rk, qi);
    sub(sub(sub(kinetic, coupling), potential), energy)
}

impl<R: AzReal> HgSystem<R> {
    /// The system of masses `m` with the frozen energy `h`, the floor `tiny` and the time transformation `time`.
    #[inline]
    pub fn new(m: [R; 3], h: R, tiny: R, time: HeggieTime) -> Self {
        let pair = |j: R, k: R| (j * k / (j + k), j * k);
        let (mu0, mm0) = pair(m[1], m[2]);
        let (mu1, mm1) = pair(m[2], m[0]);
        let (mu2, mm2) = pair(m[0], m[1]);
        HgSystem {
            m,
            total: m[0] + m[1] + m[2],
            mu: [mu0, mu1, mu2],
            mm: [mm0, mm1, mm2],
            inv_m: [R::one() / m[0], R::one() / m[1], R::one() / m[2]],
            h,
            tiny,
            time,
        }
    }

    /// The factors of index `i`, `(j, k)` its cyclic successors.
    #[inline]
    fn cyc(&self, i: u32) -> Cyc<R> {
        let (a, b, c) = (self.inv_m[0], self.inv_m[1], self.inv_m[2]);
        if i == 0 {
            Cyc {
                mu_i: self.mu[0],
                inv_mi: a,
                inv_mj: b,
                inv_mk: c,
                mm_j: self.mm[1],
                mm_k: self.mm[2],
            }
        } else if i == 1 {
            Cyc {
                mu_i: self.mu[1],
                inv_mi: b,
                inv_mj: c,
                inv_mk: a,
                mm_j: self.mm[2],
                mm_k: self.mm[0],
            }
        } else {
            Cyc {
                mu_i: self.mu[2],
                inv_mi: c,
                inv_mj: a,
                inv_mk: b,
                mm_j: self.mm[0],
                mm_k: self.mm[1],
            }
        }
    }

    /// `S = R₀ + R₁ + R₂` floored at `TINY` (Eq. 22's).
    #[inline]
    fn s_floored(&self, r: [R; 3]) -> R {
        floored(r[0] + r[1] + r[2], self.tiny)
    }

    /// `dt/dτ` under the time transformation in force, unfloored itself: `R₀R₁R₂` under Eq. 20, `R₀R₁R₂/S^{3/2}` under
    /// Eq. 22, `S` floored at `TINY` (Part 2b; `driver.rs:203-215`).
    #[inline]
    pub fn dt_dtau(&self, q: &HgQ<R>) -> R {
        let r = rr(q);
        let product = r[0] * r[1] * r[2];
        match self.time {
            HeggieTime::Eq20 => product,
            HeggieTime::Eq22 => {
                let s = self.s_floored(r);
                product / (s * s.square_root())
            }
        }
    }

    /// `Γ* = Σᵢ RⱼRₖ|Pᵢ|²/(8μᵢ) − Σᵢ Rᵢ Wⱼ·Wₖ/(4mᵢ) − Σᵢ mⱼmₖRⱼRₖ − h·R₀R₁R₂` (Part 2b), summed in that order.
    #[inline]
    pub fn gamma(&self, q: &HgQ<R>, p: &HgP<R>) -> R {
        let r = rr(q);
        let w = ws(q, p);
        let four = two::<R>() * two::<R>();
        let eight = two::<R>() * four;
        let kinetic = r[1] * r[2] * dot(p.p[0], p.p[0]) / (eight * self.mu[0])
            + r[2] * r[0] * dot(p.p[1], p.p[1]) / (eight * self.mu[1])
            + r[0] * r[1] * dot(p.p[2], p.p[2]) / (eight * self.mu[2]);
        let coupling = r[0] * self.inv_m[0] * dot(w[1], w[2]) / four
            + r[1] * self.inv_m[1] * dot(w[2], w[0]) / four
            + r[2] * self.inv_m[2] * dot(w[0], w[1]) / four;
        let potential =
            self.mm[0] * r[1] * r[2] + self.mm[1] * r[2] * r[0] + self.mm[2] * r[0] * r[1];
        kinetic - coupling - potential - self.h * r[0] * r[1] * r[2]
    }

    /// The rates under Eq. 20 (Part 2b, Eq. 21): `dQᵢ/dτ = ∂Γ*/∂Pᵢ`, `dPᵢ/dτ = −∂Γ*/∂Qᵢ`, `dt/dτ = R₀R₁R₂`.
    #[inline]
    fn rates_eq20(&self, q: &HgQ<R>, p: &HgP<R>) -> (HgQ<R>, HgP<R>) {
        let r = rr(q);
        let w = ws(q, p);
        let (c0, c1, c2) = (self.cyc(0), self.cyc(1), self.cyc(2));
        let others = |pj, pk, wj, wk, rj, rk, mu_j, mu_k| Others {
            pj,
            pk,
            wj,
            wk,
            rj,
            rk,
            mu_j,
            mu_k,
        };
        let o0 = others(
            p.p[1], p.p[2], w[1], w[2], r[1], r[2], self.mu[1], self.mu[2],
        );
        let o1 = others(
            p.p[2], p.p[0], w[2], w[0], r[2], r[0], self.mu[2], self.mu[0],
        );
        let o2 = others(
            p.p[0], p.p[1], w[0], w[1], r[0], r[1], self.mu[0], self.mu[1],
        );
        let neg = |v: V<R>| scale(-R::one(), v);
        (
            HgQ {
                q: [
                    dgamma_dp(q.q[0], p.p[0], w[1], w[2], r[1], r[2], c0),
                    dgamma_dp(q.q[1], p.p[1], w[2], w[0], r[2], r[0], c1),
                    dgamma_dp(q.q[2], p.p[2], w[0], w[1], r[0], r[1], c2),
                ],
                t: r[0] * r[1] * r[2],
            },
            HgP {
                p: [
                    neg(dgamma_dq(q.q[0], p.p[0], o0, c0, self.h)),
                    neg(dgamma_dq(q.q[1], p.p[1], o1, c1, self.h)),
                    neg(dgamma_dq(q.q[2], p.p[2], o2, c2, self.h)),
                ],
            },
        )
    }

    /// The rates under the time transformation in force (Part 2b): Eq. 20's as they stand; under Eq. 22 each scaled by
    /// `S^{−3/2}`, the momentum rates first gaining `3Γ*Qᵢ/S` (Heggie's Eqs. 23, 24), `S` floored at `TINY`.
    #[inline]
    pub fn rates_of(&self, q: &HgQ<R>, p: &HgP<R>) -> (HgQ<R>, HgP<R>) {
        let (dq, dp) = self.rates_eq20(q, p);
        match self.time {
            HeggieTime::Eq20 => (dq, dp),
            HeggieTime::Eq22 => {
                let s = self.s_floored(rr(q));
                let f = R::one() / (s * s.square_root());
                let control = (two::<R>() + R::one()) * self.gamma(q, p) / s;
                let term = |dpi: V<R>, qi: V<R>| scale(f, add(dpi, scale(control, qi)));
                (
                    HgQ {
                        q: [scale(f, dq.q[0]), scale(f, dq.q[1]), scale(f, dq.q[2])],
                        t: dq.t * f,
                    },
                    HgP {
                        p: [
                            term(dp.p[0], q.q[0]),
                            term(dp.p[1], q.q[1]),
                            term(dp.p[2], q.q[2]),
                        ],
                    },
                )
            }
        }
    }

    /// The relative positions and momenta `(qᵢ, pᵢ)` of the regularised state: `qᵢ = L(Qᵢ)Qᵢ`,
    /// `pᵢ = L(Qᵢ)Pᵢ/(2·max(Rᵢ, TINY))` (Part 2b; `system.rs:153-168`).
    #[inline]
    pub fn phys(&self, q: &HgQ<R>, p: &HgP<R>) -> ([V<R>; 3], [V<R>; 3]) {
        let r = rr(q);
        let back = |qi: V<R>, pi: V<R>, ri: R| {
            scale(
                R::one() / (two::<R>() * floored(ri, self.tiny)),
                lmul(qi, pi),
            )
        };
        (
            [lc(q.q[0]), lc(q.q[1]), lc(q.q[2])],
            [
                back(q.q[0], p.p[0], r[0]),
                back(q.q[1], p.p[1], r[1]),
                back(q.q[2], p.p[2], r[2]),
            ],
        )
    }

    /// Heggie's Eq. 6 in the centre-of-mass frame on the relative `(q, p)`:
    /// `Σᵢ [|pᵢ|²/(2μᵢ) − pⱼ·pₖ/mᵢ − mⱼmₖ/max(|qᵢ|, TINY)]` (Part 2b; `system.rs:170-186`).
    #[inline]
    pub fn energy_relative(&self, q: &[V<R>; 3], p: &[V<R>; 3]) -> R {
        let two = two::<R>();
        let term = |pi: V<R>, pj: V<R>, pk: V<R>, qi: V<R>, mu: R, inv_m: R, mm: R| {
            dot(pi, pi) / (two * mu) - dot(pj, pk) * inv_m - mm / floored(norm(qi), self.tiny)
        };
        term(
            p[0],
            p[1],
            p[2],
            q[0],
            self.mu[0],
            self.inv_m[0],
            self.mm[0],
        ) + term(
            p[1],
            p[2],
            p[0],
            q[1],
            self.mu[1],
            self.inv_m[1],
            self.mm[1],
        ) + term(
            p[2],
            p[0],
            p[1],
            q[2],
            self.mu[2],
            self.inv_m[2],
            self.mm[2],
        )
    }

    /// The regularised energy of `(q, p)` (integrator_contract Part 1, R-566; REQ-INT-104): Eq. 6 on [`HgSystem::phys`],
    /// read from the regularised coordinates and momenta (`system.rs:188-192`).
    #[inline]
    pub fn energy(&self, q: &HgQ<R>, p: &HgP<R>) -> R {
        let (qv, pv) = self.phys(q, p);
        self.energy_relative(&qv, &pv)
    }

    /// The relative velocities of Heggie's Eq. 7, `q̇ᵢ = pᵢ/μᵢ − pₖ/mⱼ − pⱼ/mₖ` (`driver.rs:217-228`).
    #[inline]
    pub fn q_dot(&self, p: &[V<R>; 3]) -> [V<R>; 3] {
        let one = |pi: V<R>, pj: V<R>, pk: V<R>, mu: R, inv_mj: R, inv_mk: R| {
            sub(
                sub(scale(R::one() / mu, pi), scale(inv_mj, pk)),
                scale(inv_mk, pj),
            )
        };
        let [a, b, c] = self.inv_m;
        [
            one(p[0], p[1], p[2], self.mu[0], b, c),
            one(p[1], p[2], p[0], self.mu[1], c, a),
            one(p[2], p[0], p[1], self.mu[2], a, b),
        ]
    }

    /// The Cartesian state of `(q, p)` (Part 2b; Heggie Eqs. 10, 12): `rᵢ = (mⱼqₖ − mₖqⱼ)/M`, the momenta
    /// `mᵢvᵢ = pₖ − pⱼ`, in the centre-of-mass frame.
    #[inline]
    pub fn image(&self, q: &HgQ<R>, p: &HgP<R>) -> State<R> {
        let (qv, pv) = self.phys(q, p);
        let [m0, m1, m2] = self.m;
        let inv = R::one() / self.total;
        let r = |mj: R, qk: V<R>, mk: R, qj: V<R>| scale(inv, sub(scale(mj, qk), scale(mk, qj)));
        State {
            m: self.m,
            r: [
                r(m1, qv[2], m2, qv[1]),
                r(m2, qv[0], m0, qv[2]),
                r(m0, qv[1], m1, qv[0]),
            ],
            p: [sub(pv[2], pv[1]), sub(pv[0], pv[2]), sub(pv[1], pv[0])],
        }
    }
}

/// A registration: the system with its frozen `h`, and the regularised state at `t = 0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HgRegistered<R> {
    /// The system.
    pub system: HgSystem<R>,
    /// The regularised positions, the clock at `0`.
    pub q: HgQ<R>,
    /// The regularised momenta.
    pub p: HgP<R>,
}

/// Registers the Cartesian state `s`, in the centre-of-mass frame, under `time` (Part 2b; `system.rs:82-100`,
/// `:137-151`): `qᵢ = rⱼ − rₖ`, `pᵢ = (mⱼvⱼ − mₖvₖ)/3` (Eq. 8b), the energy `h` by Eq. 6, `Qᵢ` the conditioned inverse
/// of `qᵢ`, `Pᵢ = 2L(Qᵢ)ᵀpᵢ`, and the interval clock `t = 0`.
#[inline]
pub fn register<R: AzReal>(s: &State<R>, tiny: R, time: HeggieTime) -> HgRegistered<R> {
    let third = R::one() / (two::<R>() + R::one());
    let qv = [
        sub(s.r[1], s.r[2]),
        sub(s.r[2], s.r[0]),
        sub(s.r[0], s.r[1]),
    ];
    let pv = [
        scale(third, sub(s.p[1], s.p[2])),
        scale(third, sub(s.p[2], s.p[0])),
        scale(third, sub(s.p[0], s.p[1])),
    ];
    let unfrozen = HgSystem::new(s.m, R::default(), tiny, time);
    let system = HgSystem {
        h: unfrozen.energy_relative(&qv, &pv),
        ..unfrozen
    };
    let u = [
        lc_inverse(qv[0], tiny),
        lc_inverse(qv[1], tiny),
        lc_inverse(qv[2], tiny),
    ];
    let two = two::<R>();
    HgRegistered {
        system,
        q: HgQ {
            q: u,
            t: R::default(),
        },
        p: HgP {
            p: [
                scale(two, ltmul(u[0], pv[0])),
                scale(two, ltmul(u[1], pv[1])),
                scale(two, ltmul(u[2], pv[2])),
            ],
        },
    }
}

/// The time-reversal map (Part 2b § "The time-reversal maps (REQ-INT-095)"): `(Q, P, t) → (Q, −P, −t)`, an involution.
#[inline]
pub fn reversed<R: AzReal>(q: &HgQ<R>, p: &HgP<R>) -> (HgQ<R>, HgP<R>) {
    let neg = |v: V<R>| scale(-R::one(), v);
    (
        HgQ { q: q.q, t: -q.t },
        HgP {
            p: [neg(p.p[0]), neg(p.p[1]), neg(p.p[2])],
        },
    )
}

impl<R: AzReal> Flow<R> for HgSystem<R> {
    type Q = HgQ<R>;
    type P = HgP<R>;

    #[inline]
    fn q_rate(&self, q: &HgQ<R>, p: &HgP<R>) -> HgQ<R> {
        self.rates_of(q, p).0
    }

    #[inline]
    fn p_rate(&self, q: &HgQ<R>, p: &HgP<R>) -> HgP<R> {
        self.rates_of(q, p).1
    }

    #[inline]
    fn rates(&self, q: &HgQ<R>, p: &HgP<R>) -> (HgQ<R>, HgP<R>) {
        self.rates_of(q, p)
    }

    #[inline]
    fn q_add(q: &HgQ<R>, h: R, dq: &HgQ<R>) -> HgQ<R> {
        HgQ {
            q: [
                add(q.q[0], scale(h, dq.q[0])),
                add(q.q[1], scale(h, dq.q[1])),
                add(q.q[2], scale(h, dq.q[2])),
            ],
            t: q.t + h * dq.t,
        }
    }

    #[inline]
    fn p_add(p: &HgP<R>, h: R, dp: &HgP<R>) -> HgP<R> {
        HgP {
            p: [
                add(p.p[0], scale(h, dp.p[0])),
                add(p.p[1], scale(h, dp.p[1])),
                add(p.p[2], scale(h, dp.p[2])),
            ],
        }
    }

    #[inline]
    fn q_weighted(two: R, k: [&HgQ<R>; 4]) -> HgQ<R> {
        let w =
            |a: V<R>, b: V<R>, c: V<R>, d: V<R>| add(add(add(a, scale(two, b)), scale(two, c)), d);
        HgQ {
            q: [
                w(k[0].q[0], k[1].q[0], k[2].q[0], k[3].q[0]),
                w(k[0].q[1], k[1].q[1], k[2].q[1], k[3].q[1]),
                w(k[0].q[2], k[1].q[2], k[2].q[2], k[3].q[2]),
            ],
            t: k[0].t + two * k[1].t + two * k[2].t + k[3].t,
        }
    }

    #[inline]
    fn p_weighted(two: R, k: [&HgP<R>; 4]) -> HgP<R> {
        let w =
            |a: V<R>, b: V<R>, c: V<R>, d: V<R>| add(add(add(a, scale(two, b)), scale(two, c)), d);
        HgP {
            p: [
                w(k[0].p[0], k[1].p[0], k[2].p[0], k[3].p[0]),
                w(k[0].p[1], k[1].p[1], k[2].p[1], k[3].p[1]),
                w(k[0].p[2], k[1].p[2], k[2].p[2], k[3].p[2]),
            ],
        }
    }
}

// ── The stepper slot ───────────────────────────────────────────────────────────────────────────────────────────

/// The stepper slot under Heggie: a step `h` in fictitious time on the regularised system, and the pair's profile.
/// Every [`Stepper`] fills it through its form ([`super::form::Form`]; Part 2b, R-561).
pub trait HgStepper {
    /// The profile of the pair this stepper forms with Heggie.
    fn pair_profile() -> CapabilityProfile;

    /// One step `h` from `(q, p)` on the system.
    fn regularised_step<R: AzReal>(
        system: &HgSystem<R>,
        q: &HgQ<R>,
        p: &HgP<R>,
        h: R,
    ) -> (HgQ<R>, HgP<R>);
}

impl<S: Stepper> HgStepper for S {
    #[inline]
    fn pair_profile() -> CapabilityProfile {
        S::FORM.pair_profile()
    }

    #[inline]
    fn regularised_step<R: AzReal>(
        system: &HgSystem<R>,
        q: &HgQ<R>,
        p: &HgP<R>,
        h: R,
    ) -> (HgQ<R>, HgP<R>) {
        S::FORM.step(system, q, p, h)
    }
}

/// The profile of Heggie × `S` (Part 2b: the composed occupant's profile is the pair's, its form's).
#[inline]
pub fn profile<S: HgStepper>() -> CapabilityProfile {
    S::pair_profile()
}

// ── The march ──────────────────────────────────────────────────────────────────────────────────────────────────

/// The values Heggie's step control reads (integrator_contract Part 2b, Part 3), each supplied by the caller.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HgParams<R> {
    /// The step factor `η` (REQ-INT-080, a calibration).
    pub eta: R,
    /// The sync quantum `Δ_sync`, the interval `dt_left` (R-565; REQ-INT-080, a calibration).
    pub delta_sync: R,
    /// The floor `TINY` (Part 3, Part 6; REQ-INT-105, a calibration).
    pub tiny: R,
    /// The predictive limit's `f` (R-160: 0.02); the limit is off where it is not positive.
    pub predictive_f: R,
    /// The landing tolerance, relative to `dt_left`, in the reference's place of `LAND_EPS_REL` (RQ-812, open).
    pub land_eps_rel: R,
    /// The step budget's rate, steps per unit physical time per copy (R-564; REQ-INT-103, a calibration).
    pub step_budget_rate: R,
    /// The step budget's floor `N_min` (R-604; REQ-INT-108, a calibration).
    pub n_min: u32,
    /// Whether the landing clamp is on (the default); with it off the landing tolerance is 0 (`driver.rs:328`).
    pub clamp: bool,
    /// The landing secant's most retakes (`land_max_iters`, 4); `0` turns it off.
    pub secant_tries: u32,
    /// The time transformation ([`DEFAULT_TIME`], R-161).
    pub time: HeggieTime,
}

/// One trajectory's counts across its march.
#[derive(Clone, Copy, Debug)]
pub struct HgCount<R> {
    /// The running total of steps since `t = 0`, every stepper call and secant retake counted (R-604's).
    pub steps: u32,
    /// The steps the cadence followed: those the march kept.
    pub accepted: u32,
    /// The steps on which the floor acts: `dt/dτ < TINY`, `S < TINY` under Eq. 22, or any `Rᵢ < TINY` (Part 6).
    pub floored: u32,
    /// The steps whose per-step size asked for more than the entry cap, `η·dt_left/d > dτ_entry` (Part 6).
    pub cap_hits: u32,
    /// The steps whose size the predictive limit cut.
    pub predictive_bound: u32,
    /// The intervals whose entry `dt/dτ` was not positive or not finite, which end the march.
    pub non_finite_entries: u32,
    /// The steps that carried the interval clock past `2·dt_left` with the clamp on.
    pub overshoots: u32,
    /// The most steps taken in one interval, for the record.
    pub largest_interval: u32,
    /// The largest physical step (Part 2c rule 4).
    pub dt_max: DtMax<R>,
    /// How the march stands.
    pub end: End,
}

impl<R: AzReal> HgCount<R> {
    /// A trajectory at `t = 0`: no step, still marching.
    #[inline]
    pub fn new() -> Self {
        HgCount {
            steps: 0,
            accepted: 0,
            floored: 0,
            cap_hits: 0,
            predictive_bound: 0,
            non_finite_entries: 0,
            overshoots: 0,
            largest_interval: 0,
            dt_max: DtMax::new(),
            end: End::Marching,
        }
    }
}

impl<R: AzReal> Default for HgCount<R> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// What [`march`] reached.
#[derive(Clone, Copy, Debug)]
pub struct HgAdvanced<R> {
    /// The Cartesian state: at the boundary reached, or latched where the march ended.
    pub state: State<R>,
    /// The force evaluation at `state`'s positions.
    pub geom: Geom<R>,
    /// The last boundary landed on, `k`.
    pub boundary: u32,
    /// The physical time of `state`: `k·Δ_sync` on a landing, exactly; where the march ended between boundaries,
    /// `k·Δ_sync + min(t, Δ_sync)`.
    pub t: R,
    /// The trajectory's counts.
    pub count: HgCount<R>,
    /// The system, registered once at `t = 0`, with its frozen `h`.
    pub system: HgSystem<R>,
    /// The regularised positions and the interval clock where the march ended.
    pub q: HgQ<R>,
    /// The regularised momenta where the march ended.
    pub p: HgP<R>,
}

impl<R: AzReal> HgAdvanced<R> {
    /// The regularised energy where the march ended (integrator_contract Part 1, R-566).
    #[inline]
    pub fn regularised(&self) -> R {
        self.system.energy(&self.q, &self.p)
    }
}

/// What the march asks and tells as it goes: the explicit finiteness test it exits on, a slot so that its control can
/// remove it, and, for a measurement, the registration and each step's pre-floor values.
pub trait HgWatch<R> {
    /// Whether the state `(q, p)` is finite.
    fn finite(&mut self, q: &HgQ<R>, p: &HgP<R>) -> bool;

    /// The registration's regularised positions.
    #[inline]
    fn registered(&mut self, _q: &HgQ<R>) {}

    /// A step's pre-floor `dt/dτ`, its `S` and its least `Rᵢ`.
    #[inline]
    fn sized(&mut self, _dt_dtau: R, _s: R, _r_least: R) {}
}

impl<R: AzReal> HgWatch<R> for ExplicitTest {
    #[inline]
    fn finite(&mut self, q: &HgQ<R>, p: &HgP<R>) -> bool {
        let v = |x: V<R>| finite(x[0]) && finite(x[1]);
        v(q.q[0]) && v(q.q[1]) && v(q.q[2]) && finite(q.t) && v(p.p[0]) && v(p.p[1]) && v(p.p[2])
    }
}

/// One step's size and the signals its rule counts (Part 2b, step 5; Part 6).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HgStepSize<R> {
    /// The fictitious-time step `dτ`, after the landing clamp.
    pub h: R,
    /// The floor signal: `dt/dτ < TINY`, `S < TINY` under Eq. 22, or any `Rᵢ < TINY`.
    pub floored: bool,
    /// The entry-cap hit: `η·dt_left/d > dτ_entry`.
    pub cap_hit: bool,
    /// Whether the predictive limit cut the size.
    pub by_limit: bool,
}

/// The predictive limit (Part 2b, step 6): `f·d_min/(v_max·dt/dτ)`, `d_min = min |qᵢ|`, `v_max = max |q̇ᵢ|`, and
/// whether it is in force: not where `f ≤ 0`, the denominator is not positive, or `d_min` or the denominator is not
/// finite (`driver.rs:230-252`), the absence of a bound.
#[inline]
pub fn predictive<R: AzReal>(system: &HgSystem<R>, q: &HgQ<R>, p: &HgP<R>, f: R) -> (R, bool) {
    let zero = R::default();
    if f <= zero {
        return (zero, false);
    }
    let (qv, pv) = system.phys(q, p);
    let v = system.q_dot(&pv);
    let d_min = least(least(norm(qv[0]), norm(qv[1])), norm(qv[2]));
    let v_max = floored(floored(norm(v[0]), norm(v[1])), norm(v[2]));
    let denom = v_max * system.dt_dtau(q);
    let on = denom > zero && finite(d_min) && finite(denom);
    (f * d_min / denom, on)
}

/// The size of the step from `(q, p)` in an interval whose entry size is `entry` (Part 2b, step 5): the floor signal;
/// `d = max(dt/dτ, TINY)`; `min(η·dt_left/d, entry)`, a cap hit where the first is the larger; cut by the predictive
/// limit where it is smaller; then, with the clamp on, `min(dτ, max(dt_left − t, 0)/d)`.
#[inline]
pub fn step_size<R: AzReal>(
    system: &HgSystem<R>,
    q: &HgQ<R>,
    p: &HgP<R>,
    entry: R,
    params: &HgParams<R>,
) -> HgStepSize<R> {
    let r = rr(q);
    let tiny = params.tiny;
    let dt_dtau = system.dt_dtau(q);
    // Part 6's `S < TINY` under Eq. 22 is counted by the `Rᵢ` arm: each `Rᵢ ≥ 0`, so `S < TINY` holds only where every
    // `Rᵢ < TINY` does.
    let floored_signal = dt_dtau < tiny || r[0] < tiny || r[1] < tiny || r[2] < tiny;
    let d = floored(dt_dtau, tiny);
    let c0 = params.eta * params.delta_sync / d;
    let mut h = least(c0, entry);
    let mut by_limit = false;
    let (lim, on) = predictive(system, q, p, params.predictive_f);
    if on && lim < h {
        h = lim;
        by_limit = true;
    }
    if params.clamp {
        h = least(h, floored(params.delta_sync - q.t, R::default()) / d);
    }
    HgStepSize {
        h,
        floored: floored_signal,
        cap_hit: c0 > entry,
        by_limit,
    }
}

/// What one interval's march left: the last finite state and whether it landed.
struct Interval<R> {
    q: HgQ<R>,
    p: HgP<R>,
    landed: bool,
}

/// Marches one sync interval from `(q, p)`, its clock at `0`, at boundary `k` (Part 2b, steps 2 to 9), the cadence
/// after every kept step on its Cartesian image, the counts in `count`.
#[inline]
fn interval<S: HgStepper, R: AzReal, C: Cadence<R>, X: HgWatch<R>>(
    system: &HgSystem<R>,
    (q, p): (HgQ<R>, HgP<R>),
    k: u32,
    params: &HgParams<R>,
    count: &mut HgCount<R>,
    cadence: &mut C,
    watch: &mut X,
) -> Interval<R> {
    let dt_left = params.delta_sync;
    // RQ-812: the landing tolerance is the caller's setting, no value here; the landing follows RQ-812's ruling.
    let land_eps = if params.clamp {
        params.land_eps_rel
    } else {
        R::default()
    };
    let land_at = dt_left - dt_left * land_eps;
    let land_tol = dt_left * land_eps;
    let t0 = R::count(k) * dt_left;
    let entry_rate = system.dt_dtau(&q);
    if !(finite(entry_rate) && entry_rate > R::default()) {
        count.non_finite_entries += 1;
        count.end = End::NonFinite;
        return Interval {
            q,
            p,
            landed: false,
        };
    }
    let entry = params.eta * dt_left / entry_rate;
    let (mut q, mut p) = (q, p);
    let (mut kept_q, mut kept_p) = (q, p);
    let mut stepped = false;
    let mut landed = false;
    let mut go = true;
    let mut here = 0u32;
    while go {
        if watch.finite(&q, &p) {
            if stepped {
                let mut s = system.image(&q, &p);
                let mut g = Geom::new(&s.m, &s.r);
                cadence.after_step(&mut s, &mut g);
                count.accepted += 1;
            }
            kept_q = q;
            kept_p = p;
            landed = q.t >= land_at; // RQ-812: the landing reads the supplied tolerance.
            if over_allowance(count.steps, t0 + q.t, params.step_budget_rate, params.n_min) {
                count.end = End::Exhausted;
                go = false;
            } else if cadence.done() {
                count.end = End::Done;
                go = false;
            } else if landed {
                go = false;
            }
        } else {
            count.end = End::NonFinite;
            go = false;
        }
        if go {
            let r = rr(&q);
            watch.sized(
                system.dt_dtau(&q),
                r[0] + r[1] + r[2],
                least(least(r[0], r[1]), r[2]),
            );
            let sized = step_size(system, &q, &p, entry, params);
            count.floored += u32::from(sized.floored);
            count.cap_hits += u32::from(sized.cap_hit);
            count.predictive_bound += u32::from(sized.by_limit);
            let h = sized.h;
            let before = q.t;
            let (mut q1, mut p1) = S::regularised_step(system, &q, &p, h);
            count.steps += 1;
            here += 1;
            // RQ-812: the secant's trigger and stop read the supplied tolerance.
            if q1.t >= land_at {
                let want = dt_left - before;
                let mut cur = h;
                let mut tries = 0;
                let mut retake = true;
                while retake && tries < params.secant_tries {
                    let got = q1.t - before;
                    if !finite(got) || got <= R::default() || abs(got - want) <= land_tol {
                        retake = false;
                    } else {
                        cur = cur * want / got;
                        (q1, p1) = S::regularised_step(system, &q, &p, cur);
                        count.steps += 1;
                        here += 1;
                        tries += 1;
                    }
                }
            }
            count.dt_max = count.dt_max.observe(q1.t - before);
            if params.clamp && q1.t > two::<R>() * dt_left {
                count.overshoots += 1;
            }
            q = q1;
            p = p1;
            stepped = true;
        }
    }
    count.largest_interval = count.largest_interval.max(here);
    Interval {
        q: kept_q,
        p: kept_p,
        landed: landed && (count.end == End::Marching || count.end == End::Done),
    }
}

/// Heggie's march over the sync grid (R-565), registered once (Part 2b, step 1): the Cartesian `state` at `t = 0`,
/// in the centre-of-mass frame, registered under `params.time` and marched to boundary `k_target` with the stepper `S`
/// and the watch `watch`, each boundary a sampling cadence that does not touch the regularised state or `h`. On a
/// landing the time is the boundary's, `k·Δ_sync`, exactly.
#[inline]
pub fn march<S: HgStepper, R: AzReal, C: Cadence<R>, X: HgWatch<R>>(
    state: &State<R>,
    k_target: u32,
    params: &HgParams<R>,
    cadence: &mut C,
    watch: &mut X,
) -> HgAdvanced<R> {
    let HgRegistered {
        system,
        q: q0,
        p: p0,
    } = register(state, params.tiny, params.time);
    watch.registered(&q0);
    let mut s = *state;
    let (mut q, mut p) = (q0, p0);
    let mut k = 0;
    let mut count = HgCount::new();
    let mut t_s = R::default();
    while k < k_target && count.end == End::Marching {
        q.t = R::default();
        let out = interval::<S, R, C, X>(&system, (q, p), k, params, &mut count, cadence, watch);
        q = out.q;
        p = out.p;
        s = system.image(&q, &p);
        if out.landed {
            k += 1;
        } else {
            t_s = least(q.t, params.delta_sync);
        }
    }
    HgAdvanced {
        state: s,
        geom: Geom::new(&s.m, &s.r),
        boundary: k,
        t: R::count(k) * params.delta_sync + t_s,
        count,
        system,
        q,
        p,
    }
}
