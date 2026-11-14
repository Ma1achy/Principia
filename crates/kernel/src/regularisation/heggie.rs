//! Heggie's global regularisation, the regularisation occupant of integrator_contract Part 2b § "Heggie: the equations,
//! the time transformations, the maps and the step control (transcribed)" (D. C. Heggie, Celest. Mech. 10 (1974)
//! 217–241; R-160, R-161). Planar, `G = 1`.
//!
//! **Three relative vectors, no reference body, registered once.** `qᵢ = rⱼ − rₖ`, `(j, k)` the cyclic successors of
//! `i`, each Levi-Civita regularised (`Qᵢ`, the conditioned inverse of [`super::az::lc_inverse`]) with its momentum
//! `Pᵢ = 2L(Qᵢ)ᵀpᵢ` ([`register`]); the state `(Q₀, P₀, Q₁, P₁, Q₂, P₂, t)` is marched in fictitious time `τ` under
//! `Γ*` with the energy `h` frozen at registration ([`HgSystem`], its [`Flow`]), and its Cartesian image is
//! [`HgSystem::image`]. The stepper slot, the step control and the direct march, registered once at `t = 0`, are
//! TASK-M3-77's; how the occupant seam carries the state across `ADVANCE` calls is R-992's, built by TASK-M3-75.
//!
//! **The time transformations** ([`HeggieTime`]; R-161): Eq. 20, `dτ = dt/(R₀R₁R₂)`, and Eq. 22 at n = 3/2, the
//! default ([`DEFAULT_TIME`]), `dτ = S^{3/2}/(R₀R₁R₂) dt`, whose momentum rates gain `3Γ*Qᵢ/S`, `S` floored at `TINY`.
//!
//! **The regularised energy** ([`HgSystem::energy`]; integrator_contract Part 1, R-566, REQ-INT-104): Heggie's Eq. 6
//! on the relative positions and momenta the regularised state gives, read from `(Q, P)`, never through the Cartesian
//! return. **The time-reversal map** ([`reversed`]; Part 2b, REQ-INT-095): `(Q, P, t) → (Q, −P, −t)`, `h` held.
//!
//! No precision-dependent constant is in a branch here (Part 2c rule 1): `TINY`, the one excepted floor, is the
//! caller's ([`HgSystem::tiny`]).

use super::az::{add, dot, floored, lc, lc_inverse, lmul, ltmul, norm, scale, sub, two, AzReal, V};
use super::form::Flow;
use super::pairs::HeggieTime;
use crate::occupant::State;

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
        self.cartesian(&qv, &pv)
    }

    /// The Cartesian state of the relative positions and momenta `(qv, pv)`, [`HgSystem::image`]'s second half, linear
    /// in them: `rᵢ = (mⱼqₖ − mₖqⱼ)/M` and `mᵢvᵢ = pₖ − pⱼ`. The FTLE shadow's displacement reads it on the relative
    /// displacements ([`super::heggie_joint::cartesian`]).
    #[inline]
    pub fn cartesian(&self, qv: &[V<R>; 3], pv: &[V<R>; 3]) -> State<R> {
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
