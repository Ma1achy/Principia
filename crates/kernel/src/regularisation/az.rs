//! Aarseth–Zare, the regularisation occupant of integrator_contract Part 2b § "Aarseth–Zare: the equations, the
//! reference body, the Levi-Civita inverse and the step control (transcribed)" (Aarseth & Zare 1974, Celest. Mech.
//! 10, 185; R-160, R-927, R-953). Planar, `G = 1`.
//!
//! **Two pairs about a reference body, re-registered at every sync boundary.** At each boundary the reference body `a`
//! is re-chosen from the Cartesian state ([`reference_body`]: the body not in the longest of the pairs `(0,1)`,
//! `(0,2)`, `(1,2)`, the first maximum winning a tie), the other two being `b < c`; the pairs `(a,b)` and `(a,c)` are
//! Levi-Civita regularised, the side `(b,c)` is not ([`register`]). The interval is marched in fictitious time `τ` on
//! `(u₁, p₁, u₂, p₂, t)` with the energy `E` held ([`AzSystem`], its [`Flow`]), and mapped back at its end
//! ([`AzSystem::image`]). Each registration is counted per trajectory ([`AzCount::re_registrations`]), with each change of
//! reference body ([`AzCount::switches`]).
//!
//! **The Levi-Civita inverse is the conditioned branch** ([`lc_inverse`]): the larger component is computed directly and
//! the smaller derived from it, so its accuracy does not depend on the configuration's orientation (REQ-INT-044). The
//! unstable form it replaces is kept, [`lc_inverse_unstable`], only to measure what it loses. Their guards compare with
//! the floor `TINY`, Part 2c rule 1's one exception (R-953).
//!
//! **The step control** ([`march`]), `driver.rs`'s as Part 2b transcribes it, on the fixed sync grid `t = k·Δ_sync`
//! (R-565): the interval `dt_left` is `Δ_sync`, the entry size `dτ_entry = η·dt_left/(max(A₀, TINY)·max(B₀, TINY))`;
//! each step's size the least of the finite, positive constraints `c₀ = η·dt_left/ab`, `c₁ = dτ_entry` and the
//! predictive limit `c₂ = f·d_min/(P_max·ab)` on the relative momenta `P₁, P₂` (R-953), `ab` the floored `A·B`; then
//! the landing clamp `dτ = min(dτ, max(dt_left − t_s, 0)/ab)`; one step of the stepper; the landing secant (at most
//! [`AzParams::secant_tries`] retakes, each a counted step); and the exit, when the interval clock lands, the state is
//! not finite (tested explicitly on its bits, step 7: the guard that catches a non-finite entry size, the doubly
//! degenerate state among them), or the step budget is spent. The floor signal (`A < TINY` or `B < TINY`) and the
//! entry-cap hit (`c₀ > c₁`) are counted, non-terminal signals (Part 6, R-563). Each step's physical increment is fed
//! to the trajectory's `dt_max` ([`super::dt_max`]; step 8).
//!
//! **No precision-dependent constant in a branch** (Part 2c rule 1). The interval skip of step 1 (`SYNC_EPS`) is an
//! integer test on the grid's boundary index; the landing tolerance (`LAND_EPS_REL`), whose value Part 2b leaves out of
//! the transcription, is [`AzParams::land_eps_rel`], supplied by the caller (RQ-795). `TINY` is the one excepted floor.
//!
//! **The step budget** (Part 6; R-604): a copy's running total of steps, every RK4 call and secant retake counted,
//! is checked at every step against `max(N_min, step_budget_rate·t)`, `t` its physical time since `t = 0`; a copy that
//! exceeds it ends undetermined, never run on. Read at the boundaries, it is REQ-INT-054's bound (R-953), which adds no
//! behaviour. The running total crosses calls in [`AzCount`]; the seam's call ([`Regularisation::advance`]) starts one
//! at `t_now`, the payload's budget word being TASK-M3-41's (R-603).
//!
//! **The stepper slot under Aarseth–Zare** is [`AzStepper`]: every stepper fills it through its form on the
//! regularised system ([`super::form`]), RK4's profile `symplectic: false` (Part 2a's table), and a time-transformed
//! leapfrog in Mikkola–Tanikawa form fills it as a type of its own, with Part 2a's row for the pair (order 2, one
//! evaluation, symplectic, step-reversible, not run-reversible, owning its time mapping); its equations are not in the
//! corpus and none is built (R-162; REQ-INT-024).
//!
//! The values the step control reads (`η`, `Δ_sync`, `TINY`, `N_min`, the budget's rate, the landing tolerance) are
//! calibrations (R-71: REQ-INT-080, REQ-INT-105, REQ-INT-108, REQ-INT-103; RQ-795), so the occupant reads them from its
//! [`AzConfig`], never from a literal here.

use core::marker::PhantomData;

use super::dt_max::DtMax;
use super::form::Flow;
use super::{Kind, Regularisation};
use crate::driver::advance::{AdvanceParams, AdvanceReal, Advanced, Cadence, Reached};
use crate::occupant::{CapabilityProfile, Geom, State, Stepper};
use crate::physics::PairGeom;

/// The float type Aarseth–Zare runs at: an occupant's.
pub trait AzReal: AdvanceReal {}

impl<R: AdvanceReal> AzReal for R {}

/// A 2-vector.
pub(super) type V<R> = [R; 2];

#[inline]
pub(super) fn add<R: AzReal>(a: V<R>, b: V<R>) -> V<R> {
    [a[0] + b[0], a[1] + b[1]]
}

#[inline]
pub(super) fn sub<R: AzReal>(a: V<R>, b: V<R>) -> V<R> {
    [a[0] - b[0], a[1] - b[1]]
}

#[inline]
pub(super) fn scale<R: AzReal>(s: R, a: V<R>) -> V<R> {
    [s * a[0], s * a[1]]
}

#[inline]
pub(super) fn dot<R: AzReal>(a: V<R>, b: V<R>) -> R {
    a[0] * b[0] + a[1] * b[1]
}

#[inline]
pub(super) fn norm<R: AzReal>(a: V<R>) -> R {
    dot(a, a).square_root()
}

/// `L(u)w = (u₀w₀ − u₁w₁, u₁w₀ + u₀w₁)`.
#[inline]
pub(super) fn lmul<R: AzReal>(u: V<R>, w: V<R>) -> V<R> {
    [u[0] * w[0] - u[1] * w[1], u[1] * w[0] + u[0] * w[1]]
}

/// `L(u)ᵀw = (u₀w₀ + u₁w₁, −u₁w₀ + u₀w₁)`.
#[inline]
pub(super) fn ltmul<R: AzReal>(u: V<R>, w: V<R>) -> V<R> {
    [u[0] * w[0] + u[1] * w[1], u[0] * w[1] - u[1] * w[0]]
}

/// `max(x, floor)`, a NaN `x` kept.
#[inline]
pub(super) fn floored<R: AzReal>(x: R, floor: R) -> R {
    if floor > x {
        floor
    } else {
        x
    }
}

/// `min(a, b)`, a NaN `a` kept.
#[inline]
pub(super) fn least<R: AzReal>(a: R, b: R) -> R {
    if b < a {
        b
    } else {
        a
    }
}

/// `|x|`.
#[inline]
pub(super) fn abs<R: AzReal>(x: R) -> R {
    if x < R::default() {
        -x
    } else {
        x
    }
}

/// `2` at `R`.
#[inline]
pub(super) fn two<R: AzReal>() -> R {
    R::one() + R::one()
}

/// Whether `x` is finite, on its bits (Part 2c; R-114).
#[inline]
pub(super) fn finite<R: AzReal>(x: R) -> bool {
    !x.non_finite_bits()
}

// ── The Levi-Civita map ────────────────────────────────────────────────────────────────────────────────────────

/// The Levi-Civita map `R = L(u)u = (u₀² − u₁², 2u₀u₁)` (Part 2b).
#[inline]
pub fn lc<R: AzReal>(u: V<R>) -> V<R> {
    lmul(u, u)
}

/// The two magnitudes of the inverse, `a = √max((r + x)/2, 0) = |u₀|` and `b = √max((r − x)/2, 0) = |u₁|`, `r = |R|`.
#[inline]
fn halves<R: AzReal>(r: V<R>) -> (R, R) {
    let m = norm(r);
    let zero = R::default();
    let a = floored((m + r[0]) / two(), zero).square_root();
    let b = floored((m - r[0]) / two(), zero).square_root();
    (a, b)
}

/// The conditioned Levi-Civita inverse, `u` from `R` (Part 2b, the default; REQ-INT-044): on `x ≥ 0`, `u₀ = a` and
/// `u₁ = y/(2a)` if `a > TINY`, otherwise `b`; on `x < 0`, `u₁ = −b` if `y < 0`, otherwise `b`, and `u₀ = y/(2u₁)` if
/// `|u₁| > TINY`, otherwise `a`. Both give `u₀ ≥ 0`.
#[inline]
pub fn lc_inverse<R: AzReal>(r: V<R>, tiny: R) -> V<R> {
    let (a, b) = halves(r);
    let (x, y) = (r[0], r[1]);
    if x >= R::default() {
        let u1 = if a > tiny { y / (two::<R>() * a) } else { b };
        [a, u1]
    } else {
        let u1 = if y < R::default() { -b } else { b };
        let u0 = if abs(u1) > tiny {
            y / (two::<R>() * u1)
        } else {
            a
        };
        [u0, u1]
    }
}

/// The unstable inverse the conditioned one replaces, kept only to measure its loss (Part 2b): `u₀ = a`, then
/// `u₁ = y/(2u₀)` if `u₀ > TINY`, otherwise `b`. It cancels catastrophically as `R` turns towards `−x`.
#[inline]
pub fn lc_inverse_unstable<R: AzReal>(r: V<R>, tiny: R) -> V<R> {
    let (a, b) = halves(r);
    let u1 = if a > tiny { r[1] / (two::<R>() * a) } else { b };
    [a, u1]
}

// ── The reference body ─────────────────────────────────────────────────────────────────────────────────────────

/// The reference body `a` for the positions `r` (Part 2b): the body not in the longest of the pairs taken in the order
/// `(0,1), (0,2), (1,2)`, the first maximum winning a tie (a strict `>`), so `a = [2, 1, 0][argmax]`. The pairs are
/// compared by their `d²` ([`PairGeom::d2`]), comparison-only (Part 2c rule 2), which orders them as their distances.
#[inline]
pub fn reference_body<R: AzReal>(r: &[[R; 2]; 3]) -> u32 {
    // R-22's pair ids: d2[2] is (0,1), d2[1] is (2,0), d2[0] is (1,2).
    let d2 = PairGeom::new(r).d2();
    let mut longest = d2[2];
    let mut a = 2;
    if d2[1] > longest {
        longest = d2[1];
        a = 1;
    }
    if d2[0] > longest {
        a = 0;
    }
    a
}

/// `v[i]`, read at a constant index.
#[inline]
fn pick<T: Copy>(v: &[T; 3], i: u32) -> T {
    if i == 0 {
        v[0]
    } else if i == 1 {
        v[1]
    } else {
        v[2]
    }
}

/// The bodies `(b, c)` other than `a`, `b < c`.
#[inline]
const fn others(a: u32) -> (u32, u32) {
    if a == 0 {
        (1, 2)
    } else if a == 1 {
        (0, 2)
    } else {
        (0, 1)
    }
}

/// The three bodies' values `(x_a, x_b, x_c)` put back at their indices.
#[inline]
fn place<T: Copy>(a: u32, xa: T, xb: T, xc: T) -> [T; 3] {
    if a == 0 {
        [xa, xb, xc]
    } else if a == 1 {
        [xb, xa, xc]
    } else {
        [xb, xc, xa]
    }
}

// ── The regularised system ─────────────────────────────────────────────────────────────────────────────────────

/// The regularised positions and the interval clock: `(u₁, u₂, t)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AzQ<R> {
    /// `u₁`, the pair `(a, b)`.
    pub u1: V<R>,
    /// `u₂`, the pair `(a, c)`.
    pub u2: V<R>,
    /// The interval clock `t`, physical time since the interval's boundary.
    pub t: R,
}

/// The regularised momenta `(p₁, p₂)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AzP<R> {
    /// `p₁ = 2L₁ᵀP₁`.
    pub p1: V<R>,
    /// `p₂ = 2L₂ᵀP₂`.
    pub p2: V<R>,
}

/// One interval's registration: the reference body, the masses and their factors, and the energy `E` held through it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AzSystem<R> {
    /// The reference body `a`.
    pub a: u32,
    /// The masses, by body index.
    pub m: [R; 3],
    /// `m_a`, `m_b`, `m_c`.
    pub mabc: [R; 3],
    /// `μ₁ = m_a m_b/(m_a + m_b)` and `μ₂ = m_a m_c/(m_a + m_c)`.
    pub mu: [R; 2],
    /// The velocity-space mass matrix: `K₁₁ = m_b(m_a + m_c)/M`, `K₂₂ = m_c(m_a + m_b)/M`, `K₁₂ = −m_b m_c/M`.
    pub k: [R; 3],
    /// The physical energy `E`, held through the interval.
    pub e: R,
    /// The floor `TINY`.
    pub tiny: R,
    /// The equations of motion's floor on `|R₃|` ([`AzParams::r3_floor`]).
    pub r3_floor: R,
}

impl<R: AzReal> AzSystem<R> {
    /// `A = |u₁|²` and `B = |u₂|²`, unfloored.
    #[inline]
    pub fn ab(q: &AzQ<R>) -> (R, R) {
        (dot(q.u1, q.u1), dot(q.u2, q.u2))
    }

    /// The relative positions and momenta `(R₁, R₂, P₁, P₂)`: `R = L(u)u`, `P₁ = L₁p₁/(2A)`, `P₂ = L₂p₂/(2B)`, `A` and
    /// `B` floored at `TINY` (Part 2b; R-953).
    #[inline]
    pub fn phys(&self, q: &AzQ<R>, p: &AzP<R>) -> [V<R>; 4] {
        let (a, b) = Self::ab(q);
        let a = floored(a, self.tiny);
        let b = floored(b, self.tiny);
        [
            lc(q.u1),
            lc(q.u2),
            scale(R::one() / (two::<R>() * a), lmul(q.u1, p.p1)),
            scale(R::one() / (two::<R>() * b), lmul(q.u2, p.p2)),
        ]
    }

    /// The regularised energy of `(q, p)` (integrator_contract Part 1, R-566; REQ-INT-104): the physical energy of the
    /// relative positions and momenta the regularised state gives ([`AzSystem::phys`]), each separation floored at
    /// `TINY`, read from the regularised coordinates and momenta, never through the Cartesian return.
    #[inline]
    pub fn energy(&self, q: &AzQ<R>, p: &AzP<R>) -> R {
        energy_of(self.mabc, self.mu, self.phys(q, p), self.tiny)
    }

    /// The Cartesian state the regularised `(q, p)` maps back to (Part 2b, "Registration and return"): `V₁`, `V₂` from
    /// `P₁`, `P₂` through `K`; `r_a = 0`, `r_b = R₁`, `r_c = R₂`, `v_a = 0`, `v_b = V₁`, `v_c = V₂`; then shifted to the
    /// centre of mass; the momenta `pᵢ = mᵢvᵢ`.
    #[inline]
    pub fn image(&self, q: &AzQ<R>, p: &AzP<R>) -> State<R> {
        let [r1, r2, p1, p2] = self.phys(q, p);
        let [k11, k22, k12] = self.k;
        let det = k11 * k22 - k12 * k12;
        let v1 = scale(R::one() / det, sub(scale(k22, p1), scale(k12, p2)));
        let v2 = scale(R::one() / det, sub(scale(k11, p2), scale(k12, p1)));
        let [ma, mb, mc] = self.mabc;
        let total = ma + mb + mc;
        let rc = scale(R::one() / total, add(scale(mb, r1), scale(mc, r2)));
        let vc = scale(R::one() / total, add(scale(mb, v1), scale(mc, v2)));
        let zero = [R::default(); 2];
        let r = place(self.a, sub(zero, rc), sub(r1, rc), sub(r2, rc));
        let v = place(self.a, sub(zero, vc), sub(v1, vc), sub(v2, vc));
        let m = self.m;
        State {
            m,
            r,
            p: [scale(m[0], v[0]), scale(m[1], v[1]), scale(m[2], v[2])],
        }
    }
}

/// The physical energy at the relative positions and momenta `[R₁, R₂, P₁, P₂]` (Part 2b, "Coordinates and mass
/// factors"): `E = |P₁|²/(2μ₁) + P₁·P₂/m_a + |P₂|²/(2μ₂) − m_a m_b/|R₁| − m_a m_c/|R₂| − m_b m_c/|R₃|`, each `|Rᵢ|`
/// floored at `TINY` (`system.rs`'s `energy_phys`; R-953).
#[inline]
fn energy_of<R: AzReal>(mabc: [R; 3], mu: [R; 2], [r1, r2, p1, p2]: [V<R>; 4], tiny: R) -> R {
    let [ma, mb, mc] = mabc;
    let two = two::<R>();
    let kinetic = dot(p1, p1) / (two * mu[0]) + dot(p1, p2) / ma + dot(p2, p2) / (two * mu[1]);
    let potential = ma * mb / floored(norm(r1), tiny)
        + ma * mc / floored(norm(r2), tiny)
        + mb * mc / floored(norm(sub(r2, r1)), tiny);
    kinetic - potential
}

/// Registers the Cartesian state `s` (Part 2b, "Registration and return"): the reference body ([`reference_body`]),
/// `R₁ = r_b − r_a`, `R₂ = r_c − r_a`, `V₁ = v_b − v_a`, `V₂ = v_c − v_a` (`v = p/m`), the momenta `P = K·V`, the energy
/// `E` with each `|Rᵢ|` floored at `TINY`, `u` by the conditioned inverse, `p₁ = 2L₁ᵀP₁`, `p₂ = 2L₂ᵀP₂`, and the
/// interval clock `t = 0`.
#[inline]
pub fn register<R: AzReal>(s: &State<R>, tiny: R, r3_floor: R) -> (AzSystem<R>, AzQ<R>, AzP<R>) {
    let a = reference_body(&s.r);
    let (b, c) = others(a);
    let (ma, mb, mc) = (pick(&s.m, a), pick(&s.m, b), pick(&s.m, c));
    let v = |i: u32| scale(R::one() / pick(&s.m, i), pick(&s.p, i));
    let (ra, va) = (pick(&s.r, a), v(a));
    let r1 = sub(pick(&s.r, b), ra);
    let r2 = sub(pick(&s.r, c), ra);
    let v1 = sub(v(b), va);
    let v2 = sub(v(c), va);
    let total = ma + mb + mc;
    let mu = [ma * mb / (ma + mb), ma * mc / (ma + mc)];
    let k = [
        mb * (ma + mc) / total,
        mc * (ma + mb) / total,
        -(mb * mc) / total,
    ];
    let p1 = add(scale(k[0], v1), scale(k[2], v2));
    let p2 = add(scale(k[2], v1), scale(k[1], v2));
    let two = two::<R>();
    let e = energy_of([ma, mb, mc], mu, [r1, r2, p1, p2], tiny);
    let u1 = lc_inverse(r1, tiny);
    let u2 = lc_inverse(r2, tiny);
    let system = AzSystem {
        a,
        m: s.m,
        mabc: [ma, mb, mc],
        mu,
        k,
        e,
        tiny,
        r3_floor,
    };
    let q = AzQ {
        u1,
        u2,
        t: R::default(),
    };
    let p = AzP {
        p1: scale(two, ltmul(u1, p1)),
        p2: scale(two, ltmul(u2, p2)),
    };
    (system, q, p)
}

impl<R: AzReal> Flow<R> for AzSystem<R> {
    type Q = AzQ<R>;
    type P = AzP<R>;

    /// `du₁/dτ = B·p₁/(4μ₁) + L₁ᵀ(L₂p₂)/(4m_a)`, `du₂/dτ = A·p₂/(4μ₂) + L₂ᵀ(L₁p₁)/(4m_a)`, `dt/dτ = A·B` (Part 2b).
    #[inline]
    fn q_rate(&self, q: &AzQ<R>, p: &AzP<R>) -> AzQ<R> {
        let (a, b) = Self::ab(q);
        let four = two::<R>() * two::<R>();
        let ma4 = R::one() / (four * self.mabc[0]);
        AzQ {
            u1: add(
                scale(b / (four * self.mu[0]), p.p1),
                scale(ma4, ltmul(q.u1, lmul(q.u2, p.p2))),
            ),
            u2: add(
                scale(a / (four * self.mu[1]), p.p2),
                scale(ma4, ltmul(q.u2, lmul(q.u1, p.p1))),
            ),
            t: a * b,
        }
    }

    /// `dp₁/dτ = −∂Γ/∂u₁` and `dp₂/dτ = −∂Γ/∂u₂` (Part 2b), `A` and `B` unfloored and `|R₃|` floored at
    /// [`AzSystem::r3_floor`]:
    ///
    /// ```text
    /// ∂Γ/∂u₁ = u₁|p₂|²/(4μ₂) + L(p₁)ᵀ(L₂p₂)/(4m_a) − 2m_a m_c u₁ − m_b m_c (2u₁B/|R₃| + 2(A·B/|R₃|³) L₁ᵀR₃) − 2E·B·u₁
    /// ∂Γ/∂u₂ = u₂|p₁|²/(4μ₁) + L(p₂)ᵀ(L₁p₁)/(4m_a) − 2m_a m_b u₂ − m_b m_c (2u₂A/|R₃| − 2(A·B/|R₃|³) L₂ᵀR₃) − 2E·A·u₂
    /// ```
    #[inline]
    fn p_rate(&self, q: &AzQ<R>, p: &AzP<R>) -> AzP<R> {
        let (a, b) = Self::ab(q);
        let two = two::<R>();
        let four = two * two;
        let [ma, mb, mc] = self.mabc;
        let r3 = sub(lc(q.u2), lc(q.u1));
        let n3 = floored(norm(r3), self.r3_floor);
        let cube = a * b / (n3 * n3 * n3);
        let l1p1 = lmul(q.u1, p.p1);
        let l2p2 = lmul(q.u2, p.p2);
        let ma4 = R::one() / (four * ma);
        let mbc = mb * mc;
        let g1 = sub(
            sub(
                add(
                    scale(dot(p.p2, p.p2) / (four * self.mu[1]), q.u1),
                    scale(ma4, ltmul(p.p1, l2p2)),
                ),
                scale(two * ma * mc, q.u1),
            ),
            add(
                scale(
                    mbc,
                    add(
                        scale(two * b / n3, q.u1),
                        scale(two * cube, ltmul(q.u1, r3)),
                    ),
                ),
                scale(two * self.e * b, q.u1),
            ),
        );
        let g2 = sub(
            sub(
                add(
                    scale(dot(p.p1, p.p1) / (four * self.mu[0]), q.u2),
                    scale(ma4, ltmul(p.p2, l1p1)),
                ),
                scale(two * ma * mb, q.u2),
            ),
            add(
                scale(
                    mbc,
                    sub(
                        scale(two * a / n3, q.u2),
                        scale(two * cube, ltmul(q.u2, r3)),
                    ),
                ),
                scale(two * self.e * a, q.u2),
            ),
        );
        AzP {
            p1: scale(-R::one(), g1),
            p2: scale(-R::one(), g2),
        }
    }

    #[inline]
    fn q_add(q: &AzQ<R>, h: R, dq: &AzQ<R>) -> AzQ<R> {
        AzQ {
            u1: add(q.u1, scale(h, dq.u1)),
            u2: add(q.u2, scale(h, dq.u2)),
            t: q.t + h * dq.t,
        }
    }

    #[inline]
    fn p_add(p: &AzP<R>, h: R, dp: &AzP<R>) -> AzP<R> {
        AzP {
            p1: add(p.p1, scale(h, dp.p1)),
            p2: add(p.p2, scale(h, dp.p2)),
        }
    }

    #[inline]
    fn q_weighted(two: R, k: [&AzQ<R>; 4]) -> AzQ<R> {
        let w = |a: R, b: R, c: R, d: R| a + two * b + two * c + d;
        let v =
            |f: fn(&AzQ<R>) -> V<R>, i: usize| w(f(k[0])[i], f(k[1])[i], f(k[2])[i], f(k[3])[i]);
        AzQ {
            u1: [v(|x| x.u1, 0), v(|x| x.u1, 1)],
            u2: [v(|x| x.u2, 0), v(|x| x.u2, 1)],
            t: w(k[0].t, k[1].t, k[2].t, k[3].t),
        }
    }

    #[inline]
    fn p_weighted(two: R, k: [&AzP<R>; 4]) -> AzP<R> {
        let w = |a: R, b: R, c: R, d: R| a + two * b + two * c + d;
        let v =
            |f: fn(&AzP<R>) -> V<R>, i: usize| w(f(k[0])[i], f(k[1])[i], f(k[2])[i], f(k[3])[i]);
        AzP {
            p1: [v(|x| x.p1, 0), v(|x| x.p1, 1)],
            p2: [v(|x| x.p2, 0), v(|x| x.p2, 1)],
        }
    }
}

// ── The stepper slot ───────────────────────────────────────────────────────────────────────────────────────────

/// The stepper slot under Aarseth–Zare (REQ-INT-024): a step `h` in fictitious time on the regularised system, and the
/// pair's profile. Every [`Stepper`] fills it through its form ([`super::form::Form`]); a time-transformed leapfrog in
/// Mikkola–Tanikawa form fills it as a type of its own, its profile Part 2a's row for the pair.
pub trait AzStepper {
    /// The profile of the pair this stepper forms with Aarseth–Zare.
    fn pair_profile() -> CapabilityProfile;

    /// One step `h` from `(q, p)` on the interval's system.
    fn regularised_step<R: AzReal>(
        system: &AzSystem<R>,
        q: &AzQ<R>,
        p: &AzP<R>,
        h: R,
    ) -> (AzQ<R>, AzP<R>);
}

impl<S: Stepper> AzStepper for S {
    #[inline]
    fn pair_profile() -> CapabilityProfile {
        S::FORM.pair_profile()
    }

    #[inline]
    fn regularised_step<R: AzReal>(
        system: &AzSystem<R>,
        q: &AzQ<R>,
        p: &AzP<R>,
        h: R,
    ) -> (AzQ<R>, AzP<R>) {
        S::FORM.step(system, q, p, h)
    }
}

// ── The march ──────────────────────────────────────────────────────────────────────────────────────────────────

/// The values Aarseth–Zare's step control reads (integrator_contract Part 2b, Part 3), each from the occupant's
/// [`AzConfig`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AzParams<R> {
    /// The step factor `η` (REQ-INT-080, a calibration).
    pub eta: R,
    /// The sync quantum `Δ_sync`, the interval `dt_left` (R-565; REQ-INT-080, a calibration).
    pub delta_sync: R,
    /// The floor `TINY` (Part 3, Part 6; REQ-INT-105, a calibration).
    pub tiny: R,
    /// The predictive limit's `f` (R-160: 0.02); the limit is off where it is not positive.
    pub predictive_f: R,
    /// The equations of motion's floor on `|R₃|` (Part 2b: `1e-300`, after `tb_az.py`, an f64 value; the corpus gives
    /// none at f32, RQ-796, a calibration, R-71).
    pub r3_floor: R,
    /// The landing tolerance, relative to `dt_left`, the reference's `LAND_EPS_REL` (RQ-795): one value, read where the
    /// landing is tested and where the secant stops.
    pub land_eps_rel: R,
    /// The step budget's rate, steps per unit physical time per copy (R-564; REQ-INT-103, a calibration).
    pub step_budget_rate: R,
    /// The step budget's floor `N_min` (R-604; REQ-INT-108, a calibration).
    pub n_min: u32,
    /// Whether the landing clamp is on (Part 2b, step 3; the default). Off only to measure the closure without the
    /// landing fix (REQ-INT-049); with it off the landing tolerance is 0, as the reference driver has it, and
    /// [`AzParams::land_eps_rel`] is not read.
    pub clamp: bool,
    /// The landing secant's most retakes (Part 2b, step 6: `land_max_iters`, 4); `0` turns it off.
    pub secant_tries: u32,
}

/// How an Aarseth–Zare copy's march stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    /// Still marching: its last interval landed on its boundary.
    Marching,
    /// The state was not finite at the explicit test (Part 2b, step 7): the copy stopped, its state latched at the
    /// last finite one.
    NonFinite,
    /// The step budget was spent (R-604): the copy is undetermined, never run on.
    Exhausted,
    /// The cadence's `done` ended the march: a terminal fired (Part 1).
    Done,
}

/// One trajectory's counts across its march, carried from call to call.
#[derive(Clone, Copy, Debug)]
pub struct AzCount<R> {
    /// The running total of steps since `t = 0`, every stepper call and secant retake counted (R-604's).
    pub steps: u32,
    /// The steps the cadence followed: those the march kept, the secant's last retake in place of its others.
    pub accepted: u32,
    /// The registrations, one at each sync boundary the march left from (Part 2b, "The profile gains a field").
    pub re_registrations: u32,
    /// The registrations whose reference body differs from the previous one's.
    pub switches: u32,
    /// The steps on which `A < TINY` or `B < TINY` (Part 6's floor signal).
    pub floored: u32,
    /// The steps whose per-step size asked for more than the entry cap, `c₀ > c₁` (Part 6).
    pub cap_hits: u32,
    /// The steps whose size the predictive limit set, below the other constraints.
    pub predictive_bound: u32,
    /// The intervals whose entry size `dτ_entry` was not finite.
    pub non_finite_entries: u32,
    /// The steps that carried the interval clock past `2·dt_left` (Part 2b, step 7).
    pub overshoots: u32,
    /// The most steps taken in one interval, for the record (REQ-INT-054).
    pub largest_interval: u32,
    /// The largest physical step (Part 2b, step 8; Part 2c).
    pub dt_max: DtMax<R>,
    /// The last registration's reference body; `3` before the first.
    pub reference: u32,
    /// How the march stands.
    pub end: End,
}

impl<R: AzReal> AzCount<R> {
    /// A trajectory at `t = 0`: no step, no registration, still marching.
    #[inline]
    pub fn new() -> Self {
        AzCount {
            steps: 0,
            accepted: 0,
            re_registrations: 0,
            switches: 0,
            floored: 0,
            cap_hits: 0,
            predictive_bound: 0,
            non_finite_entries: 0,
            overshoots: 0,
            largest_interval: 0,
            dt_max: DtMax::new(),
            reference: 3,
            end: End::Marching,
        }
    }
}

impl<R: AzReal> Default for AzCount<R> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// What [`march`] reached.
#[derive(Clone, Copy, Debug)]
pub struct AzAdvanced<R> {
    /// The Cartesian state: at the boundary reached, or latched where the march ended.
    pub state: State<R>,
    /// The force evaluation at `state`'s positions.
    pub geom: Geom<R>,
    /// The last boundary landed on, `k`.
    pub boundary: u32,
    /// The physical time of `state`: `k·Δ_sync` on a landing, exactly; where the march ended between boundaries,
    /// `k·Δ_sync + min(t_s, Δ_sync)` (Part 2b, step 7).
    pub t: R,
    /// The trajectory's counts.
    pub count: AzCount<R>,
    /// The energy `E` of the march's first registration: the physical energy at its start, against which its
    /// regularised energy drifts (integrator_contract Part 1, R-566). `0` where no interval was marched.
    pub registered: R,
    /// The regularised energy where the march ended ([`AzSystem::energy`] of its last interval's regularised state;
    /// integrator_contract Part 1, R-566; REQ-INT-104). `0` where no interval was marched.
    pub regularised: R,
}

/// What the march asks and tells as it goes: the explicit finiteness test it exits on (Part 2b, step 7), a slot so that
/// its control can remove it, and, for a measurement, each registration and each step's pre-floor `A` and `B`, which
/// the occupant's own watch ([`ExplicitTest`]) ignores.
pub trait Watch<R> {
    /// Whether the state `(q, p)` is finite.
    fn finite(&mut self, q: &AzQ<R>, p: &AzP<R>) -> bool;

    /// A registration's regularised positions, before its interval is marched.
    #[inline]
    fn registered(&mut self, _q: &AzQ<R>) {}

    /// A step's `A` and `B`, before the floor.
    #[inline]
    fn sized(&mut self, _a: R, _b: R) {}
}

/// The test the occupant runs: every component of `(u₁, p₁, u₂, p₂, t)`, on its bits.
#[derive(Clone, Copy, Debug)]
pub struct ExplicitTest;

impl<R: AzReal> Watch<R> for ExplicitTest {
    #[inline]
    fn finite(&mut self, q: &AzQ<R>, p: &AzP<R>) -> bool {
        finite(q.u1[0])
            && finite(q.u1[1])
            && finite(q.u2[0])
            && finite(q.u2[1])
            && finite(q.t)
            && finite(p.p1[0])
            && finite(p.p1[1])
            && finite(p.p2[0])
            && finite(p.p2[1])
    }
}

/// Whether a constraint is usable: finite and positive (Part 2b, step 3).
#[inline]
fn usable<R: AzReal>(c: R) -> bool {
    finite(c) && c > R::default()
}

/// The step's size before the clamp: the least of the usable constraints, and whether it was the predictive limit's;
/// where none is usable, the entry size `c₁` as it stands, which is then not finite (Part 2b, step 7).
#[inline]
fn size<R: AzReal>(c0: R, c1: R, c2: R) -> (R, bool) {
    let mut h = c1;
    let mut have = false;
    let mut by_limit = false;
    if usable(c0) {
        h = c0;
        have = true;
    }
    if usable(c1) && (!have || c1 < h) {
        h = c1;
        have = true;
    }
    if usable(c2) && (!have || c2 < h) {
        h = c2;
        by_limit = true;
    }
    (h, by_limit)
}

/// The predictive limit (Part 2b, step 4): `f·d_min/(P_max·ab)`, `d_min = min(|R₁|, |R₂|, |R₂ − R₁|)` and
/// `P_max = max(|P₁|, |P₂|, |P₂ − P₁|)` on the relative momenta (R-953). Where `f ≤ 0`, the denominator is not
/// positive, or `d_min` or the denominator is not finite, the value is not finite or not positive, so step 3's usable
/// test drops it and the limit is off.
#[inline]
fn predictive<R: AzReal>(system: &AzSystem<R>, q: &AzQ<R>, p: &AzP<R>, f: R, ab: R) -> R {
    let [r1, r2, p1, p2] = system.phys(q, p);
    let d_min = least(least(norm(r1), norm(r2)), norm(sub(r2, r1)));
    let (n1, n2, n3) = (norm(p1), norm(p2), norm(sub(p2, p1)));
    let p_max = floored(floored(n1, n2), n3);
    f * d_min / (p_max * ab)
}

/// R-604's budget for any regularised occupant: whether the running total `steps` exceeds `max(n_min, rate·t)` at
/// physical time `t`. An allowance that is not a number, read on its bits, spends the budget too, so that the march
/// stays bounded (Part 2c rule 3).
#[inline]
pub fn over_allowance<R: AzReal>(steps: u32, t: R, rate: R, n_min: u32) -> bool {
    let allowance = floored(rate * t, R::count(n_min));
    R::count(steps) > allowance || allowance.nan_bits()
}

/// Whether the running total `steps` exceeds Aarseth–Zare's budget `max(N_min, rate·t)` at physical time `t` (R-604):
/// [`over_allowance`] at its values.
#[inline]
pub fn over_budget<R: AzReal>(steps: u32, t: R, params: &AzParams<R>) -> bool {
    over_allowance(steps, t, params.step_budget_rate, params.n_min)
}

/// One step's size and the signals its rule counts (Part 2b, steps 3 and 4; Part 6).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StepSize<R> {
    /// The fictitious-time step `dτ`, after the landing clamp.
    pub h: R,
    /// The floor signal: `A < TINY` or `B < TINY`.
    pub floored: bool,
    /// The entry-cap hit: the per-step size `c₀` asked for more than the entry size `c₁`.
    pub cap_hit: bool,
    /// Whether the predictive limit set the size, below the other usable constraints.
    pub by_limit: bool,
}

/// The size of the step from `(q, p)` in an interval whose entry size is `entry` (Part 2b, steps 3 and 4): `ab` the
/// floored `A·B`; the least of the usable (finite, positive) constraints `c₀ = η·dt_left/ab`, `c₁ = entry` and the
/// predictive limit `c₂`, the entry size as it stands where none is usable; then, where the clamp is on, the landing
/// clamp `min(dτ, max(dt_left − t_s, 0)/ab)`. A state degenerate only mid-interval has a non-finite `c₀` and a finite
/// entry size, so its step is finite, a counted cap hit; one degenerate at the interval's entry has neither, and its
/// non-finite size reaches the stepper and the explicit finiteness test (step 7).
#[inline]
pub fn step_size<R: AzReal>(
    system: &AzSystem<R>,
    q: &AzQ<R>,
    p: &AzP<R>,
    entry: R,
    params: &AzParams<R>,
) -> StepSize<R> {
    let (a, b) = AzSystem::ab(q);
    let ab = floored(a, params.tiny) * floored(b, params.tiny);
    let c0 = params.eta * params.delta_sync / ab;
    let c2 = predictive(system, q, p, params.predictive_f, ab);
    let (mut h, by_limit) = size(c0, entry, c2);
    if params.clamp {
        h = least(h, floored(params.delta_sync - q.t, R::default()) / ab);
    }
    StepSize {
        h,
        floored: a < params.tiny || b < params.tiny,
        cap_hit: c0 > entry,
        by_limit,
    }
}

/// The entry size of an interval registered at `q` (Part 2b, step 2): `η·dt_left/(max(A₀, TINY)·max(B₀, TINY))`.
#[inline]
pub fn entry_size<R: AzReal>(q: &AzQ<R>, params: &AzParams<R>) -> R {
    let (a0, b0) = AzSystem::ab(q);
    params.eta * params.delta_sync / (floored(a0, params.tiny) * floored(b0, params.tiny))
}

/// What one interval's march left: the last finite state, its interval clock, and whether it landed: on its boundary
/// with the march going on, or with a terminal that fired on the landing step itself (the landing registered, then the
/// march ended).
struct Interval<R> {
    q: AzQ<R>,
    p: AzP<R>,
    landed: bool,
}

/// Marches one sync interval from the registration's `(q, p)` at boundary `k` (Part 2b, the step control, steps 2 to 8),
/// the cadence after every kept step on its Cartesian image, the counts in `count`.
#[inline]
fn interval<S: AzStepper, R: AzReal, C: Cadence<R>, X: Watch<R>>(
    system: &AzSystem<R>,
    (q, p): (AzQ<R>, AzP<R>),
    k: u32,
    params: &AzParams<R>,
    count: &mut AzCount<R>,
    cadence: &mut C,
    watch: &mut X,
) -> Interval<R> {
    let dt_left = params.delta_sync;
    // With the clamp off the landing tolerance is 0, the reference driver's (`land_tol` is 0 unless the clamp is on;
    // physics review 5873527379 of PR #502, point 2): the run without the landing fix lands only at `dt_left`.
    let land_eps = if params.clamp {
        params.land_eps_rel
    } else {
        R::default()
    };
    let land_at = dt_left - dt_left * land_eps;
    let land_tol = dt_left * land_eps;
    let t0 = R::count(k) * dt_left;
    let entry = entry_size(&q, params);
    if !finite(entry) {
        count.non_finite_entries += 1;
    }
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
            landed = q.t >= land_at;
            // `done` is read before the landing: a terminal that fires on the landing step ends the march there, its
            // state latched, never marched on into the next interval (Part 2a § "The sync grid is a fixed quantum of
            // physical time (R-565)": "a terminal event … still latches at the step it fires on"; code review
            // 5880407311 of PR #502, F2). The landing itself still counts ([`Interval::landed`]): the boundary is
            // reached, its time exact, and the march then ends (physics review 5880652590, P3; code review 5880701680,
            // F3).
            if over_budget(count.steps, t0 + q.t, params) {
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
            let (a, b) = AzSystem::ab(&q);
            watch.sized(a, b);
            let sized = step_size(system, &q, &p, entry, params);
            count.floored += u32::from(sized.floored);
            count.cap_hits += u32::from(sized.cap_hit);
            count.predictive_bound += u32::from(sized.by_limit);
            let h = sized.h;
            let before = q.t;
            let (mut q1, mut p1) = S::regularised_step(system, &q, &p, h);
            count.steps += 1;
            here += 1;
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
            if q1.t > two::<R>() * dt_left {
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

/// Aarseth–Zare's march over the sync grid (R-565): from the Cartesian `state` at boundary `k_now` to boundary
/// `k_target`, registering at each boundary, with the stepper `S`, the watch `watch` (its finiteness test) and the
/// trajectory's counts `count` carried in. A copy whose march has ended is not marched again. On a landing the time is the boundary's,
/// `k·Δ_sync`, exactly.
#[inline]
pub fn march<S: AzStepper, R: AzReal, C: Cadence<R>, X: Watch<R>>(
    state: &State<R>,
    k_now: u32,
    k_target: u32,
    count: AzCount<R>,
    params: &AzParams<R>,
    cadence: &mut C,
    watch: &mut X,
) -> AzAdvanced<R> {
    let mut s = *state;
    let mut k = k_now;
    let mut count = count;
    let mut t_s = R::default();
    let mut registered = R::default();
    let mut regularised = R::default();
    let first = count.re_registrations;
    while k < k_target && count.end == End::Marching {
        let (system, q, p) = register(&s, params.tiny, params.r3_floor);
        if count.re_registrations == first {
            registered = system.e;
        }
        count.re_registrations += 1;
        if count.reference != 3 && count.reference != system.a {
            count.switches += 1;
        }
        count.reference = system.a;
        watch.registered(&q);
        let out = interval::<S, R, C, X>(&system, (q, p), k, params, &mut count, cadence, watch);
        regularised = system.energy(&out.q, &out.p);
        s = system.image(&out.q, &out.p);
        if out.landed {
            k += 1;
        } else {
            t_s = least(out.q.t, params.delta_sync);
        }
    }
    AzAdvanced {
        state: s,
        geom: Geom::new(&s.m, &s.r),
        boundary: k,
        t: R::count(k) * params.delta_sync + t_s,
        count,
        registered,
        regularised,
    }
}

/// The latest sync boundary at or before the physical time `t`: `k` with `k·Δ_sync ≤ t < (k + 1)·Δ_sync`, decided by
/// those products, comparison-only (Part 2c rule 2); `0` for a `t` before the first boundary after `0`.
#[inline]
pub fn boundary_at_or_before<R: AzReal>(t: R, delta_sync: R) -> u32 {
    let mut k = (t / delta_sync).whole();
    if k > 0 && R::count(k) * delta_sync > t {
        k -= 1;
    }
    if k < u32::MAX && R::count(k + 1) * delta_sync <= t {
        k += 1;
    }
    k
}

/// Where Aarseth–Zare's values come from: the occupant's configuration, a type, so that the composed occupant stays a
/// type (Part 2b). Its values are calibrations (the module's note).
pub trait AzConfig {
    /// The step control's values at `R`.
    fn params<R: AzReal>() -> AzParams<R>;
}

/// Aarseth–Zare with the values of `C`.
#[derive(Clone, Copy, Debug)]
pub struct AarsethZare<C>(PhantomData<C>);

impl<C: AzConfig> AarsethZare<C> {
    /// `ADVANCE` in physical time (Part 2a; R-565): from `state` at the latest boundary at or before `t_now` to the
    /// latest boundary at or before `t_target`, never stopping between boundaries but where the march ends, its
    /// counts `count` carried in.
    #[inline]
    pub fn advance_to<S: AzStepper, R: AzReal, Cd: Cadence<R>>(
        state: &State<R>,
        t_now: R,
        t_target: R,
        count: AzCount<R>,
        cadence: &mut Cd,
    ) -> AzAdvanced<R> {
        let params = C::params::<R>();
        march::<S, R, Cd, ExplicitTest>(
            state,
            boundary_at_or_before(t_now, params.delta_sync),
            boundary_at_or_before(t_target, params.delta_sync),
            count,
            &params,
            cadence,
            &mut ExplicitTest,
        )
    }
}

impl<C: AzConfig> Regularisation for AarsethZare<C> {
    const KIND: Kind = Kind::AarsethZare;

    #[inline]
    fn profile<S: Stepper>() -> CapabilityProfile {
        <S as AzStepper>::pair_profile()
    }

    /// The seam's `ADVANCE`: `t_now` and `t_target` on the macro-step grid, physical time `index·dt_macro`, mapped to
    /// the sync grid ([`AarsethZare::advance_to`]); a fresh trajectory count at `t_now`. `substeps` is the steps the
    /// cadence followed.
    #[inline]
    fn advance<S: Stepper, R: AdvanceReal, Cd: Cadence<R>>(
        state: &State<R>,
        _geom: &Geom<R>,
        t_now: u32,
        t_target: u32,
        params: &AdvanceParams<R>,
        cadence: &mut Cd,
    ) -> Advanced<R> {
        advance_seam::<C, S, R, Cd>(state, t_now, t_target, params, cadence).0
    }

    /// The seam's `ADVANCE` with how the march ended: a landing on the boundary at or before `t_target`
    /// is the target reached, whatever the steps it took (Part 2a, Cost 2), and a march ended non-finite or by its
    /// budget ends the run, latched where it stopped (Part 2a § "The sync grid is a fixed quantum of physical time
    /// (R-565)").
    #[inline]
    fn advance_reaching<S: Stepper, R: AdvanceReal, Cd: Cadence<R>>(
        state: &State<R>,
        _geom: &Geom<R>,
        t_now: u32,
        t_target: u32,
        params: &AdvanceParams<R>,
        cadence: &mut Cd,
    ) -> (Advanced<R>, Reached) {
        advance_seam::<C, S, R, Cd>(state, t_now, t_target, params, cadence)
    }
}

/// How a march that ended at `end` on boundary `boundary` ended its seam call, its target boundary `target`:
/// [`End::Marching`], every interval landed up to the boundary at or before the target, is the target reached;
/// [`End::Done`] is the target reached where the terminal fired on the target boundary's landing step, as a stepper's
/// terminal on its last substep completes the macro-step, and short of it otherwise; [`End::NonFinite`] and
/// [`End::Exhausted`] end the copy's march.
#[inline]
fn reached(end: End, boundary: u32, target: u32) -> Reached {
    match end {
        End::Marching => Reached::Target,
        End::Done => {
            if boundary == target {
                Reached::Target
            } else {
                Reached::Short
            }
        }
        End::NonFinite => Reached::NonFinite,
        End::Exhausted => Reached::Exhausted,
    }
}

/// [`AarsethZare`]'s seam `ADVANCE`, with how its march ended ([`reached`]).
#[inline]
fn advance_seam<C: AzConfig, S: Stepper, R: AdvanceReal, Cd: Cadence<R>>(
    state: &State<R>,
    t_now: u32,
    t_target: u32,
    params: &AdvanceParams<R>,
    cadence: &mut Cd,
) -> (Advanced<R>, Reached) {
    let t_target = R::count(t_target) * params.dt_macro;
    let out = AarsethZare::<C>::advance_to::<S, R, Cd>(
        state,
        R::count(t_now) * params.dt_macro,
        t_target,
        AzCount::new(),
        cadence,
    );
    let target = boundary_at_or_before(t_target, C::params::<R>().delta_sync);
    (
        Advanced {
            state: out.state,
            geom: out.geom,
            substeps: out.count.accepted,
        },
        reached(out.count.end, out.boundary, target),
    )
}
