//! logH, the algorithmic regularisation: a time transformation and nothing else (integrator_contract Part 2b § "logH
//! and TTL: the equations, the time transformations, the map and the step control (transcribed)"; Mikkola & Tanikawa,
//! MNRAS 310 (1999) 745; Preto & Tremaine, AJ 118 (1999) 2532; the TTL mode after Mikkola & Aarseth, CMDA 84 (2002)
//! 343, as the reference set cites it). Planar, `G = 1`. Transcribed from `docs/reference/prin-rs/src/integrate/logh/`.
//!
//! **No chart.** The state is the Cartesian one, `(r, p)`, with the interval clock `t` and the auxiliary `W`
//! ([`LhQ`], [`LhP`]); there is no coordinate transformation, no reference body and nothing to re-register, so the
//! Cartesian image ([`LhSystem::image`]) is a read. The reference carries velocities; the kernel's state carries
//! momenta, so the equations are written on `p`, `v = p/m` formed where a velocity is read.
//!
//! **The two time modes** ([`LogHMode`]), each a pair of denominators, one under the drift and one under the kick:
//! - logH: `Λ = ln(K + B) − ln(U)`, `U = Σ mᵢmⱼ/rᵢⱼ > 0`, `K` the kinetic energy and `B = U − K = −E` frozen at
//!   registration. The drift divides by `K + B`, the kick by `U`.
//! - TTL: the drift divides by the carried `W`, the kick by `Ω(r) = w·Σ 1/rᵢⱼ`, `w = m̄²`, `m̄` the mean mass; `W` is
//!   registered once as `Ω(r₀)` and advanced by `dW = Ω̇ dt`. At equal masses `Ω = U`.
//!
//! On the solution the two denominators of a mode are equal; off it they differ, and a step that reads them at
//! different points (the leapfrog) is the method. `dt/ds = 1/max(drift, TINY)` ([`LhSystem::dt_ds`]), one expression
//! for the rate of `t` and for the step control.
//!
//! **The steps** ([`LhStepper`]): logH's own leapfrog, drift–kick–drift `X(h/2) V(h) X(h/2)`, the TTL kick advancing
//! `W` at the midpoint velocity and the clock advanced once a step by the two drifts' summed time ([`leapfrog`]),
//! selected as logH × KDK; and classical RK4 on `(r, p, t, W)` through the system's [`Flow`], logH × RK4.
//!
//! **The march** ([`march`]) is Heggie's step control on logH's clock ([`super::heggie`]): registered once at `t = 0`,
//! `B` and `W` carried across every boundary of the sync grid; the interval clock reset at each interval's start; an
//! entry `dt/ds` that is not positive or not finite ends the march not finite; each step's size
//! `min(η·dt_left/d, ds_entry)`, cut by the predictive limit `f·d_min/(v_max·dt/ds)`, then by the landing clamp; the
//! landing with no tolerance, a step past the boundary retaken from its saved state by a secant and then a bisection,
//! so the state stored at a boundary has the boundary's own clock; the explicit finiteness exit and the step budget
//! ([`super::az::over_allowance`]), which end the march [`End::NonFinite`] and [`End::Exhausted`] apart. The floor
//! signal (the drift denominator below `TINY`, or either denominator not positive) and the entry-cap hit are counted,
//! non-terminal.
//!
//! **The energy** ([`LhSystem::energy`]) is the Cartesian `K − U` of the state, there being no other; `K + B − U` is
//! its change since registration ([`LhSystem::residual`]). **The time-reversal map** ([`reversed`]):
//! `(r, p, t, W) → (r, −p, −t, W)`, `B` held.
//!
//! No precision-dependent constant is in a branch here: `TINY` is the one excepted floor, and the landing's only other
//! constants are its two integer caps. The step control's values are read from [`LhParams`], never from a literal.

use super::az::{
    add, dot, finite, floored, least, norm, over_allowance, scale, sub, two, AzReal, End,
    ExplicitTest, V,
};
use super::dt_max::DtMax;
use super::form::{Flow, Form};
use super::pairs::LogHMode;
use crate::driver::advance::Cadence;
use crate::occupant::kdk::Kdk;
use crate::occupant::rk4::Rk4;
use crate::occupant::{CapabilityProfile, Geom, State};
use crate::physics::energy::{kinetic, potential};
use crate::physics::force::force;
use crate::physics::PairGeom;

/// The positions and the interval clock: `(r₀, r₁, r₂, t)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LhQ<R> {
    /// The Cartesian positions.
    pub r: [V<R>; 3],
    /// The interval clock `t`, physical time since the interval's boundary.
    pub t: R,
}

/// The momenta and TTL's auxiliary variable: `(p₀, p₁, p₂, W)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LhP<R> {
    /// The Cartesian momenta `mᵢvᵢ`.
    pub p: [V<R>; 3],
    /// `W`: registered as `Ω(r₀)`, advanced by `dW = Ω̇ dt` in the TTL mode, where it divides the drift; held in the
    /// logH mode, where nothing reads it.
    pub w: R,
}

/// The system: the masses, `B` frozen at registration, the floor, the time mode and TTL's pair weight. It depends on
/// nothing but the masses and `B`, so it is built once per trajectory.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LhSystem<R> {
    /// The masses `mᵢ`.
    pub m: [R; 3],
    /// `1/mᵢ`.
    pub inv_m: [R; 3],
    /// `B = U − K = −E`, frozen at registration.
    pub b: R,
    /// The floor `TINY`.
    pub tiny: R,
    /// The time mode in force.
    pub mode: LogHMode,
    /// TTL's pair weight `w = m̄²`, `m̄ = (m₀ + m₁ + m₂)/3`.
    pub weight: R,
}

impl<R: AzReal> LhSystem<R> {
    /// The system of masses `m` with the frozen `b`, the floor `tiny` and the time mode `mode`.
    #[inline]
    pub fn new(m: [R; 3], b: R, tiny: R, mode: LogHMode) -> Self {
        let mean = (m[0] + m[1] + m[2]) / (two::<R>() + R::one());
        LhSystem {
            m,
            inv_m: [R::one() / m[0], R::one() / m[1], R::one() / m[2]],
            b,
            tiny,
            mode,
            weight: mean * mean,
        }
    }

    /// The velocities `vᵢ = pᵢ/mᵢ` of the momenta `p`.
    #[inline]
    pub fn velocities(&self, p: &[V<R>; 3]) -> [V<R>; 3] {
        [
            scale(self.inv_m[0], p[0]),
            scale(self.inv_m[1], p[1]),
            scale(self.inv_m[2], p[2]),
        ]
    }

    /// `U = Σ mᵢmⱼ/rᵢⱼ`, the potential's magnitude, from the pairwise geometry `g`.
    #[inline]
    pub fn u(&self, g: &PairGeom<R>) -> R {
        -potential(&self.m, g)
    }

    /// `Ω = w·Σ 1/max(rᵢⱼ, TINY)`, TTL's time-transformation function (`hamiltonian.rs:137-145`).
    #[inline]
    pub fn omega(&self, g: &PairGeom<R>) -> R {
        let d = g.norm();
        self.weight / floored(d[0], self.tiny)
            + self.weight / floored(d[1], self.tiny)
            + self.weight / floored(d[2], self.tiny)
    }

    /// `Ω̇ = −w·Σ (rᵢⱼ·vᵢⱼ)/max(rᵢⱼ, TINY)³` at the geometry `g` and the velocities `v`, exact, not differenced
    /// (`hamiltonian.rs:151-161`). Pair `k` joins the two bodies other than `k`.
    #[inline]
    pub fn omega_dot(&self, g: &PairGeom<R>, v: &[V<R>; 3]) -> R {
        let sep = g.sep();
        let d = g.norm();
        let term = |s: V<R>, dv: V<R>, n: R| {
            let n = floored(n, self.tiny);
            self.weight * dot(s, dv) / (n * n * n)
        };
        -(term(sep[0], sub(v[2], v[1]), d[0])
            + term(sep[1], sub(v[0], v[2]), d[1])
            + term(sep[2], sub(v[1], v[0]), d[2]))
    }

    /// The drift denominator before the floor: `K + B` in the logH mode, the carried `W` in the TTL mode
    /// (`hamiltonian.rs:164-177`). It reads the momenta and `W` only, which a drift does not change.
    #[inline]
    pub fn drift_raw(&self, p: &LhP<R>) -> R {
        match self.mode {
            LogHMode::LogH => kinetic(&self.m, &p.p) + self.b,
            LogHMode::Ttl => p.w,
        }
    }

    /// The kick denominator before the floor: `U` in the logH mode, `Ω` in the TTL mode (`hamiltonian.rs:164-177`). It
    /// reads the positions only, which a kick does not change.
    #[inline]
    pub fn kick_raw(&self, g: &PairGeom<R>) -> R {
        match self.mode {
            LogHMode::LogH => self.u(g),
            LogHMode::Ttl => self.omega(g),
        }
    }

    /// `dt/ds = 1/max(drift, TINY)` (`driver.rs:237-241`): the rate of `t` in [`Self::rates_of`] and in the leapfrog's
    /// drift, and the rate the step control sizes by, one expression, so the landing clamp sizes a step by the rate
    /// the step advances `t` at.
    #[inline]
    pub fn dt_ds(&self, p: &LhP<R>) -> R {
        R::one() / floored(self.drift_raw(p), self.tiny)
    }

    /// The kick's rate `1/max(kick, TINY)`.
    #[inline]
    fn kick_rate(&self, g: &PairGeom<R>) -> R {
        R::one() / floored(self.kick_raw(g), self.tiny)
    }

    /// The equations of motion (`hamiltonian.rs:220-248`): `drᵢ/ds = vᵢ/drift`, `dt/ds = 1/drift`,
    /// `dpᵢ/ds = mᵢaᵢ/kick`, and `dW/ds = Ω̇/kick` in the TTL mode, `0` in the logH mode; each denominator floored at
    /// `TINY`. One force evaluation.
    #[inline]
    pub fn rates_of(&self, q: &LhQ<R>, p: &LhP<R>) -> (LhQ<R>, LhP<R>) {
        let g = PairGeom::new(&q.r);
        let a = force(&self.m, &g);
        let v = self.velocities(&p.p);
        let rate = self.dt_ds(p);
        let kick = self.kick_rate(&g);
        let w = match self.mode {
            LogHMode::LogH => R::default(),
            LogHMode::Ttl => self.omega_dot(&g, &v) * kick,
        };
        (
            LhQ {
                r: [scale(rate, v[0]), scale(rate, v[1]), scale(rate, v[2])],
                t: rate,
            },
            LhP {
                p: [
                    scale(kick * self.m[0], a[0]),
                    scale(kick * self.m[1], a[1]),
                    scale(kick * self.m[2], a[2]),
                ],
                w,
            },
        )
    }

    /// The Cartesian energy `K − U` of `(q, p)`: logH's state is the physical one, so this is its only energy.
    #[inline]
    pub fn energy(&self, q: &LhQ<R>, p: &LhP<R>) -> R {
        kinetic(&self.m, &p.p) + potential(&self.m, &PairGeom::new(&q.r))
    }

    /// `(K + B − U)/U` at `(q, p)` (`hamiltonian.rs:197-205`): the energy's change since registration over `U`, not an
    /// independent constraint. Signed and unguarded, where the reference returns the magnitude, and `+∞` where `U` is
    /// not positive or not finite: a reader that needs either forms it.
    #[inline]
    pub fn residual(&self, q: &LhQ<R>, p: &LhP<R>) -> R {
        let u = self.u(&PairGeom::new(&q.r));
        (kinetic(&self.m, &p.p) + self.b - u) / u
    }

    /// The Cartesian state of `(q, p)`: a read.
    #[inline]
    pub fn image(&self, q: &LhQ<R>, p: &LhP<R>) -> State<R> {
        State {
            m: self.m,
            r: q.r,
            p: p.p,
        }
    }
}

/// A registration: the system with its frozen `B`, and the state at `t = 0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LhRegistered<R> {
    /// The system.
    pub system: LhSystem<R>,
    /// The positions, the clock at `0`.
    pub q: LhQ<R>,
    /// The momenta, `W` at `Ω(r₀)`.
    pub p: LhP<R>,
}

/// Registers the Cartesian state `s` under `mode` (`driver.rs:279-291`, `system.rs:33-36`): `B = U − K`, the clock
/// `t = 0`, and `W = Ω(r₀)`, set once in either mode as the reference sets it.
#[inline]
pub fn register<R: AzReal>(s: &State<R>, tiny: R, mode: LogHMode) -> LhRegistered<R> {
    let g = PairGeom::new(&s.r);
    let unfrozen = LhSystem::new(s.m, R::default(), tiny, mode);
    let system = LhSystem {
        b: unfrozen.u(&g) - kinetic(&s.m, &s.p),
        ..unfrozen
    };
    LhRegistered {
        system,
        q: LhQ {
            r: s.r,
            t: R::default(),
        },
        p: LhP {
            p: s.p,
            w: system.omega(&g),
        },
    }
}

/// The time-reversal map: `(r, p, t, W) → (r, −p, −t, W)`, an involution; `B` is held.
#[inline]
pub fn reversed<R: AzReal>(q: &LhQ<R>, p: &LhP<R>) -> (LhQ<R>, LhP<R>) {
    let neg = |v: V<R>| scale(-R::one(), v);
    (
        LhQ { r: q.r, t: -q.t },
        LhP {
            p: [neg(p.p[0]), neg(p.p[1]), neg(p.p[2])],
            w: p.w,
        },
    )
}

impl<R: AzReal> Flow<R> for LhSystem<R> {
    type Q = LhQ<R>;
    type P = LhP<R>;

    #[inline]
    fn q_rate(&self, q: &LhQ<R>, p: &LhP<R>) -> LhQ<R> {
        self.rates_of(q, p).0
    }

    #[inline]
    fn p_rate(&self, q: &LhQ<R>, p: &LhP<R>) -> LhP<R> {
        self.rates_of(q, p).1
    }

    #[inline]
    fn rates(&self, q: &LhQ<R>, p: &LhP<R>) -> (LhQ<R>, LhP<R>) {
        self.rates_of(q, p)
    }

    #[inline]
    fn q_add(q: &LhQ<R>, h: R, dq: &LhQ<R>) -> LhQ<R> {
        LhQ {
            r: [
                add(q.r[0], scale(h, dq.r[0])),
                add(q.r[1], scale(h, dq.r[1])),
                add(q.r[2], scale(h, dq.r[2])),
            ],
            t: q.t + h * dq.t,
        }
    }

    #[inline]
    fn p_add(p: &LhP<R>, h: R, dp: &LhP<R>) -> LhP<R> {
        LhP {
            p: [
                add(p.p[0], scale(h, dp.p[0])),
                add(p.p[1], scale(h, dp.p[1])),
                add(p.p[2], scale(h, dp.p[2])),
            ],
            w: p.w + h * dp.w,
        }
    }

    #[inline]
    fn q_weighted(two: R, k: [&LhQ<R>; 4]) -> LhQ<R> {
        let w =
            |a: V<R>, b: V<R>, c: V<R>, d: V<R>| add(add(add(a, scale(two, b)), scale(two, c)), d);
        LhQ {
            r: [
                w(k[0].r[0], k[1].r[0], k[2].r[0], k[3].r[0]),
                w(k[0].r[1], k[1].r[1], k[2].r[1], k[3].r[1]),
                w(k[0].r[2], k[1].r[2], k[2].r[2], k[3].r[2]),
            ],
            t: k[0].t + two * k[1].t + two * k[2].t + k[3].t,
        }
    }

    #[inline]
    fn p_weighted(two: R, k: [&LhP<R>; 4]) -> LhP<R> {
        let w =
            |a: V<R>, b: V<R>, c: V<R>, d: V<R>| add(add(add(a, scale(two, b)), scale(two, c)), d);
        LhP {
            p: [
                w(k[0].p[0], k[1].p[0], k[2].p[0], k[3].p[0]),
                w(k[0].p[1], k[1].p[1], k[2].p[1], k[3].p[1]),
                w(k[0].p[2], k[1].p[2], k[2].p[2], k[3].p[2]),
            ],
            w: k[0].w + two * k[1].w + two * k[2].w + k[3].w,
        }
    }
}

// ── The leapfrog ───────────────────────────────────────────────────────────────────────────────────────────────

/// The positions `r` moved by the physical time `dt` at the momenta `p`: `rᵢ + (pᵢ/mᵢ)·dt`.
#[inline]
fn moved<R: AzReal>(system: &LhSystem<R>, r: &[V<R>; 3], p: &LhP<R>, dt: R) -> [V<R>; 3] {
    let v = system.velocities(&p.p);
    [
        add(r[0], scale(dt, v[0])),
        add(r[1], scale(dt, v[1])),
        add(r[2], scale(dt, v[2])),
    ]
}

/// One drift of fictitious length `h` (`step.rs:78-84`): `dt = h·dt/ds`, `rᵢ += vᵢ dt`, `t += dt`. No force
/// evaluation: the denominator reads the momenta and `W`, which a drift does not change.
#[inline]
pub fn drift<R: AzReal>(system: &LhSystem<R>, q: &LhQ<R>, p: &LhP<R>, h: R) -> LhQ<R> {
    let dt = h * system.dt_ds(p);
    LhQ {
        r: moved(system, &q.r, p, dt),
        t: q.t + dt,
    }
}

/// One kick of fictitious length `h` (`step.rs:92-115`): `dt = h/max(kick, TINY)`, `pᵢ += mᵢaᵢ dt`, and in the TTL
/// mode `W += Ω̇(r, v_mid) dt`, `v_mid` the mean of the velocities before and after the kick: read at either end alone
/// the kick would not be symmetric under time reversal. One force evaluation.
#[inline]
pub fn kick<R: AzReal>(system: &LhSystem<R>, q: &LhQ<R>, p: &LhP<R>, h: R) -> LhP<R> {
    let g = PairGeom::new(&q.r);
    let a = force(&system.m, &g);
    let dt = h * system.kick_rate(&g);
    let kicked = [
        add(p.p[0], scale(dt * system.m[0], a[0])),
        add(p.p[1], scale(dt * system.m[1], a[1])),
        add(p.p[2], scale(dt * system.m[2], a[2])),
    ];
    let w = match system.mode {
        LogHMode::LogH => p.w,
        LogHMode::Ttl => {
            let half = R::one() / two::<R>();
            let mid = [
                scale(half, add(p.p[0], kicked[0])),
                scale(half, add(p.p[1], kicked[1])),
                scale(half, add(p.p[2], kicked[2])),
            ];
            p.w + system.omega_dot(&g, &system.velocities(&mid)) * dt
        }
    };
    LhP { p: kicked, w }
}

/// logH's own leapfrog, drift–kick–drift `X(h/2) V(h) X(h/2)` (`step.rs:122-135`), in either time mode: time-symmetric,
/// one force evaluation.
///
/// The clock is advanced once, `t += dt_a + dt_b`, the two drifts' physical times summed first. The reference adds
/// each drift's `dt` to `t` as it goes (`step.rs:84`), which is the same number until the clock is one ulp short of a
/// boundary: there each half is half an ulp, both additions round away, and a march that lands with no tolerance
/// never lands (measured on the figure-eight, `Δ_sync = T/32`: the leapfrog stood one ulp short until its budget was
/// spent). Summed first, a clamped step's increment is the whole gap, and the clock reaches the boundary. The
/// positions still drift in two halves; nothing reads the clock between them.
#[inline]
pub fn leapfrog<R: AzReal>(system: &LhSystem<R>, q: &LhQ<R>, p: &LhP<R>, h: R) -> (LhQ<R>, LhP<R>) {
    let half = h / two::<R>();
    let dt_a = half * system.dt_ds(p);
    let q_half = LhQ {
        r: moved(system, &q.r, p, dt_a),
        t: q.t,
    };
    let p1 = kick(system, &q_half, p, h);
    let dt_b = half * system.dt_ds(&p1);
    (
        LhQ {
            r: moved(system, &q_half.r, &p1, dt_b),
            t: q.t + (dt_a + dt_b),
        },
        p1,
    )
}

// ── The stepper slot ───────────────────────────────────────────────────────────────────────────────────────────

/// The stepper slot under logH: a step `h` in fictitious time, and the pair's profile in a time mode.
///
/// The plan is unclear here; chose to fill the slot for KDK and RK4 only, because the contract defines those two pairs
/// (logH × KDK is logH's own leapfrog, logH × RK4 classical RK4 on `(r, p, t, W)`) and leaves logH × Euler and the
/// Yoshidas to be written; a stepper without an impl fails to build under logH.
pub trait LhStepper {
    /// The profile of the pair this stepper forms with logH in the time mode `mode`.
    fn pair_profile(mode: LogHMode) -> CapabilityProfile;

    /// One step `h` from `(q, p)` on the system.
    fn regularised_step<R: AzReal>(
        system: &LhSystem<R>,
        q: &LhQ<R>,
        p: &LhP<R>,
        h: R,
    ) -> (LhQ<R>, LhP<R>);
}

impl LhStepper for Kdk {
    /// The leapfrog's: order 2, one force evaluation, step-reversible in either time mode (the kernel's
    /// `logh_step_reversible` test measures it), symplectic in the logH mode, where the drift and the kick are the exact
    /// flows of `ln(K + B)` and `−ln U`, and not claimed in the TTL mode, whose `W` has no Hamiltonian; the TTL mode is
    /// the one run-reversible occupant. `run_reversible` is the plan's assignment, not a measurement: no test here
    /// reverses a whole march.
    #[inline]
    fn pair_profile(mode: LogHMode) -> CapabilityProfile {
        CapabilityProfile {
            order: 2,
            force_evals: 1,
            symplectic: mode == LogHMode::LogH,
            step_reversible: true,
            run_reversible: mode == LogHMode::Ttl,
            owns_time_mapping: true,
        }
    }

    #[inline]
    fn regularised_step<R: AzReal>(
        system: &LhSystem<R>,
        q: &LhQ<R>,
        p: &LhP<R>,
        h: R,
    ) -> (LhQ<R>, LhP<R>) {
        leapfrog(system, q, p, h)
    }
}

impl LhStepper for Rk4 {
    /// RK4's form's profile on a regularised system ([`Form::pair_profile`]).
    #[inline]
    fn pair_profile(_mode: LogHMode) -> CapabilityProfile {
        Form::Rk4.pair_profile()
    }

    #[inline]
    fn regularised_step<R: AzReal>(
        system: &LhSystem<R>,
        q: &LhQ<R>,
        p: &LhP<R>,
        h: R,
    ) -> (LhQ<R>, LhP<R>) {
        Form::Rk4.step(system, q, p, h)
    }
}

/// The profile of logH × `S` in the time mode `mode`.
#[inline]
pub fn profile<S: LhStepper>(mode: LogHMode) -> CapabilityProfile {
    S::pair_profile(mode)
}

// ── The march ──────────────────────────────────────────────────────────────────────────────────────────────────

/// The values logH's step control reads, each supplied by the caller.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LhParams<R> {
    /// The step factor `η`.
    pub eta: R,
    /// The sync quantum `Δ_sync`, the interval `dt_left`.
    pub delta_sync: R,
    /// The floor `TINY`.
    pub tiny: R,
    /// The predictive limit's `f`; the limit is off where it is not positive. The reference records that the limit
    /// spends the step budget at an exact collision (`driver.rs:18-41`): `dt/ds = 1/U`, so the bound tends to
    /// `f·√d` where the transformation's own step grows as `1/d`.
    pub predictive_f: R,
    /// The step budget's rate, steps per unit physical time per copy.
    pub step_budget_rate: R,
    /// The step budget's floor `N_min`.
    pub n_min: u32,
    /// Whether the landing clamp is on (the default). Off it sizes no step to the boundary, but does not turn the
    /// landing off: a step that ends the clock past the boundary still takes the retakes. Off only for a measurement.
    pub clamp: bool,
    /// The landing secant's most retakes (the reference's `land_max_iters`, 4); `0` turns it off.
    pub secant_tries: u32,
    /// The landing bisection's most halvings. With this `0` a step past the boundary is kept, the boundary's time set
    /// over a state whose own clock is past it: a setting only for the clock-only control, never the occupant's.
    pub bisect_tries: u32,
    /// The time mode.
    pub mode: LogHMode,
}

/// One trajectory's counts across its march.
#[derive(Clone, Copy, Debug)]
pub struct LhCount<R> {
    /// The running total of steps since `t = 0`, every stepper call and landing retake counted.
    pub steps: u32,
    /// The steps the cadence followed: those the march kept.
    pub accepted: u32,
    /// The steps on which the floor acts: the drift denominator below `TINY`, or either denominator not positive.
    pub floored: u32,
    /// The steps whose per-step size asked for more than the entry cap, `η·dt_left/d > ds_entry`.
    pub cap_hits: u32,
    /// The steps whose size the predictive limit cut.
    pub predictive_bound: u32,
    /// The intervals whose entry `dt/ds` was not positive or not finite, which end the march.
    pub non_finite_entries: u32,
    /// The steps that carried the interval clock past `2·dt_left` with the clamp on.
    pub overshoots: u32,
    /// The intervals whose landing kept a step short of the boundary, closed by a further clamped step.
    pub closings: u32,
    /// The most halvings one landing took.
    pub most_bisections: u32,
    /// The most steps taken in one interval, for the record.
    pub largest_interval: u32,
    /// The largest physical step.
    pub dt_max: DtMax<R>,
    /// How the march stands.
    pub end: End,
}

impl<R: AzReal> LhCount<R> {
    /// A trajectory at `t = 0`: no step, still marching.
    #[inline]
    pub fn new() -> Self {
        LhCount {
            steps: 0,
            accepted: 0,
            floored: 0,
            cap_hits: 0,
            predictive_bound: 0,
            non_finite_entries: 0,
            overshoots: 0,
            closings: 0,
            most_bisections: 0,
            largest_interval: 0,
            dt_max: DtMax::new(),
            end: End::Marching,
        }
    }
}

impl<R: AzReal> Default for LhCount<R> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// What [`march`] reached.
#[derive(Clone, Copy, Debug)]
pub struct LhAdvanced<R> {
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
    pub count: LhCount<R>,
    /// The system, registered once at `t = 0`, with its frozen `B`.
    pub system: LhSystem<R>,
    /// The positions and the interval clock where the march ended.
    pub q: LhQ<R>,
    /// The momenta and `W` where the march ended.
    pub p: LhP<R>,
}

impl<R: AzReal> LhAdvanced<R> {
    /// The energy `K − U` where the march ended.
    #[inline]
    pub fn energy(&self) -> R {
        self.system.energy(&self.q, &self.p)
    }
}

/// What the march asks and tells as it goes: the explicit finiteness test it exits on, a slot so that its control can
/// remove it, and, for a measurement, each step's denominators before the floor.
pub trait LhWatch<R> {
    /// Whether the state `(q, p)` is finite.
    fn finite(&mut self, q: &LhQ<R>, p: &LhP<R>) -> bool;

    /// A step's drift and kick denominators, before the floor.
    #[inline]
    fn sized(&mut self, _drift: R, _kick: R) {}
}

impl<R: AzReal> LhWatch<R> for ExplicitTest {
    #[inline]
    fn finite(&mut self, q: &LhQ<R>, p: &LhP<R>) -> bool {
        let v = |x: V<R>| finite(x[0]) && finite(x[1]);
        v(q.r[0])
            && v(q.r[1])
            && v(q.r[2])
            && finite(q.t)
            && v(p.p[0])
            && v(p.p[1])
            && v(p.p[2])
            && finite(p.w)
    }
}

/// One step's size and the signals its rule counts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LhStepSize<R> {
    /// The fictitious-time step `ds`, after the landing clamp.
    pub h: R,
    /// The floor signal: the drift denominator below `TINY`, or either denominator not positive.
    pub floored: bool,
    /// The entry-cap hit: `η·dt_left/d > ds_entry`.
    pub cap_hit: bool,
    /// Whether the predictive limit cut the size.
    pub by_limit: bool,
    /// The drift denominator before the floor.
    pub drift: R,
    /// The kick denominator before the floor.
    pub kick: R,
}

/// The predictive limit (`driver.rs:243-267`): `f·d_min/(v_max·dt/ds)`, `d_min` the least pair separation of the
/// geometry `g` and `v_max` the largest relative speed `|vⱼ − vᵢ|`, read off the state, and whether it is in force:
/// not where `f ≤ 0`, the denominator is not positive, or `d_min` or the denominator is not finite, the absence of a
/// bound.
#[inline]
pub fn predictive<R: AzReal>(system: &LhSystem<R>, g: &PairGeom<R>, p: &LhP<R>, f: R) -> (R, bool) {
    let zero = R::default();
    if f <= zero {
        return (zero, false);
    }
    let v = system.velocities(&p.p);
    let d_min = g.r_min();
    let v_max = floored(
        floored(norm(sub(v[2], v[1])), norm(sub(v[0], v[2]))),
        norm(sub(v[1], v[0])),
    );
    let denom = v_max * system.dt_ds(p);
    let on = denom > zero && finite(d_min) && finite(denom);
    (f * d_min / denom, on)
}

/// The size of the step from `(q, p)` in an interval whose entry size is `entry` (`driver.rs:365-384`): the floor
/// signal; `d = dt/ds`, floored; `min(η·dt_left/d, entry)`, a cap hit where the first is the larger; cut by the
/// predictive limit where it is smaller; then, with the clamp on, `min(ds, max(dt_left − t, 0)/d)`, the same `d`.
#[inline]
pub fn step_size<R: AzReal>(
    system: &LhSystem<R>,
    q: &LhQ<R>,
    p: &LhP<R>,
    entry: R,
    params: &LhParams<R>,
) -> LhStepSize<R> {
    let zero = R::default();
    let g = PairGeom::new(&q.r);
    let drift = system.drift_raw(p);
    let kick = system.kick_raw(&g);
    let positive = drift > zero && kick > zero;
    let d = system.dt_ds(p);
    let c0 = params.eta * params.delta_sync / d;
    let mut h = least(c0, entry);
    let mut by_limit = false;
    let (lim, on) = predictive(system, &g, p, params.predictive_f);
    if on && lim < h {
        h = lim;
        by_limit = true;
    }
    if params.clamp {
        h = least(h, floored(params.delta_sync - q.t, zero) / d);
    }
    LhStepSize {
        h,
        floored: drift < params.tiny || !positive,
        cap_hit: c0 > entry,
        by_limit,
        drift,
        kick,
    }
}

/// One landing's retakes, Heggie's rule on logH's clock ([`super::heggie`]; the contract's logH step 8), from the
/// saved state `(q, p)` whose step `h` gave `(q1, p1)` past the boundary `dt_left`: the secant on `t(h)`
/// (`driver.rs:397-419`, its stop without the tolerance), at most `secant_tries` retakes, stopping where a retake ends
/// the clock on the boundary, its increment is not finite or not positive, or the tries are spent; then, where the
/// last retake is still past, its clock finite, a bisection on `h` between `0` (the saved state) and that retake's
/// step, at most `bisect_tries` halvings. The bisection keeps the bracket's short end, so the step kept ends the clock
/// at or short of the boundary, never past it. Each retake is a counted step.
///
/// This is the contract's one landing rule, Heggie's, not the reference's (`driver.rs:338`, `:390-419`: a landing test
/// and a secant stop that each read `LAND_EPS_REL`): that tolerance is a precision-dependent constant in a branch and
/// leaves the state stored at a boundary with a clock that is not the boundary's.
struct Landing<R> {
    q: LhQ<R>,
    p: LhP<R>,
    /// The retakes the secant took.
    retakes: u32,
    /// The halvings the bisection took.
    halvings: u32,
}

/// [`Landing`]'s retakes.
#[inline]
fn land<S: LhStepper, R: AzReal>(
    system: &LhSystem<R>,
    (q, p): (&LhQ<R>, &LhP<R>),
    h: R,
    (q1, p1): (LhQ<R>, LhP<R>),
    params: &LhParams<R>,
) -> Landing<R> {
    let dt_left = params.delta_sync;
    let zero = R::default();
    let before = q.t;
    let want = dt_left - before;
    let (mut q1, mut p1) = (q1, p1);
    let mut cur = h;
    let mut retakes = 0;
    let mut go = true;
    while go && retakes < params.secant_tries {
        let got = q1.t - before;
        if !finite(got) || got <= zero || q1.t == dt_left {
            go = false;
        } else {
            cur = cur * want / got;
            (q1, p1) = S::regularised_step(system, q, p, cur);
            retakes += 1;
        }
    }
    let mut halvings = 0;
    if finite(q1.t) && q1.t > dt_left {
        // The bracket: `lo` the largest step known to end the clock at or short of the boundary, with its state (`0`,
        // the saved state, to begin with); `hi` the least known to end it past (the secant's last step).
        let (mut lo, mut lo_q, mut lo_p) = (zero, *q, *p);
        let mut hi = cur;
        let two = two::<R>();
        go = true;
        while go && halvings < params.bisect_tries {
            let mid = lo + (hi - lo) / two;
            if mid <= lo || mid >= hi {
                go = false;
            } else {
                let (qm, pm) = S::regularised_step(system, q, p, mid);
                halvings += 1;
                if finite(qm.t) && qm.t <= dt_left {
                    (lo, lo_q, lo_p) = (mid, qm, pm);
                    go = qm.t < dt_left;
                } else {
                    // Past, or a clock that is not finite: never kept.
                    hi = mid;
                }
            }
        }
        if params.bisect_tries > 0 {
            (q1, p1) = (lo_q, lo_p);
        }
    }
    Landing {
        q: q1,
        p: p1,
        retakes,
        halvings,
    }
}

/// What one interval's march left: the last finite state and whether it landed.
struct Interval<R> {
    q: LhQ<R>,
    p: LhP<R>,
    landed: bool,
}

/// Marches one sync interval from `(q, p)`, its clock at `0`, at boundary `k`, the cadence after every kept step on
/// its Cartesian image, the counts in `count`. The interval lands when a kept step ends the clock at or past the
/// boundary, with no tolerance; with the landing's retakes on, a kept step never ends past it, so the state's own clock
/// is the boundary's.
#[inline]
fn interval<S: LhStepper, R: AzReal, C: Cadence<R>, X: LhWatch<R>>(
    system: &LhSystem<R>,
    (q, p): (LhQ<R>, LhP<R>),
    k: u32,
    params: &LhParams<R>,
    count: &mut LhCount<R>,
    cadence: &mut C,
    watch: &mut X,
) -> Interval<R> {
    let dt_left = params.delta_sync;
    let t0 = R::count(k) * dt_left;
    let entry_rate = system.dt_ds(&p);
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
    let mut closed = false;
    let mut go = true;
    let mut here = 0u32;
    while go {
        if watch.finite(&q, &p) {
            if stepped {
                let mut s = system.image(&q, &p);
                let mut g = Geom::new(&s.m, &s.r);
                cadence.after_step(&mut s, &mut g);
                count.accepted += 1;
                // A landing is a sampling point of the sync grid, boundary `k + 1` (Part 7: the escape window is
                // sampled at sync boundaries under a regularised occupant), on the state the cadence left, before
                // `done` is read, so a terminal it fires latches on this step.
                if q.t >= dt_left {
                    cadence.sample(&s, k + 1, dt_left);
                }
            }
            kept_q = q;
            kept_p = p;
            landed = q.t >= dt_left;
            // `done` is read before the landing: a terminal that fires on the landing step ends the march there, its
            // state latched; the landing itself still counts ([`Interval::landed`]).
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
            let sized = step_size(system, &q, &p, entry, params);
            watch.sized(sized.drift, sized.kick);
            count.floored += u32::from(sized.floored);
            count.cap_hits += u32::from(sized.cap_hit);
            count.predictive_bound += u32::from(sized.by_limit);
            let h = sized.h;
            let before = q.t;
            let (mut q1, mut p1) = S::regularised_step(system, &q, &p, h);
            count.steps += 1;
            here += 1;
            if q1.t > dt_left {
                let l = land::<S, R>(system, (&q, &p), h, (q1, p1), params);
                (q1, p1) = (l.q, l.p);
                let retakes = l.retakes + l.halvings;
                count.steps += retakes;
                here += retakes;
                count.most_bisections = count.most_bisections.max(l.halvings);
                if q1.t < dt_left && !closed {
                    count.closings += 1;
                    closed = true;
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

/// logH's direct march over the sync grid, registered once: the Cartesian `state` at `t = 0`, registered under
/// `params.mode` and marched to boundary `k_target`, the physical time `k_target·Δ_sync`, with the stepper `S` and the
/// watch `watch`, each boundary a sampling cadence that touches neither the state, `B` nor `W`. On a landing the time
/// is the boundary's, `k·Δ_sync`, exactly, and the state's own clock is `Δ_sync`.
#[inline]
pub fn march<S: LhStepper, R: AzReal, C: Cadence<R>, X: LhWatch<R>>(
    state: &State<R>,
    k_target: u32,
    params: &LhParams<R>,
    cadence: &mut C,
    watch: &mut X,
) -> LhAdvanced<R> {
    let LhRegistered {
        system,
        q: q0,
        p: p0,
    } = register(state, params.tiny, params.mode);
    let mut s = *state;
    let (mut q, mut p) = (q0, p0);
    let mut k = 0;
    let mut count = LhCount::new();
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
    LhAdvanced {
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
