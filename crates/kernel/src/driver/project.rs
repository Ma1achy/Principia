//! The COM projection after every `STEP` (integrator_contract Part 1, "COM projection and invariant monitoring";
//! dd_integrator §3.4; REQ-INT-010, REQ-INT-011, REQ-INT-030): the wrapper's housekeeping that keeps floating-point
//! drift from putting the centre of mass's translation and the bulk momentum back, so that f32's dynamic range is
//! spent on the internal motion.
//!
//! **Position–momentum form**, with the total mass `M` constant, computed once and cached ([`ComProjection::new`]):
//!
//! ```text
//! R_com = Σ mᵢrᵢ / M ;   P_com = Σ pᵢ
//! rᵢ ← rᵢ − R_com ;      pᵢ ← pᵢ − (mᵢ/M)·P_com
//! ```
//!
//! and, for an occupant that evolves velocities, `V_com = Σ mᵢvᵢ / M`, `vᵢ ← vᵢ − V_com`
//! ([`ComProjection::project_velocities`]). No occupant built so far evolves velocities: every stepper marches the
//! momenta (integrator_contract Part 2), so the wrapper's cadence calls [`ComProjection::project`].
//!
//! **CoM and linear momentum only** (REQ-INT-011). The projection is linear and removes the two modes the exact
//! translation symmetry generates; it never projects, rescales or otherwise restores the energy or `L_z`, which the
//! monitor ([`super::monitor`]) only reads. Subtracting a spurious bulk momentum does change the evaluated energy by
//! the bulk kinetic term it removes; that change stays visible in the post-projection monitoring (payload §1,
//! "Coordinate pipeline").
//!
//! **One written operation order, on the parity surface** (Part 5; REQ-INT-030). The projection is part of the
//! deterministic step, on CPU-f64, CPU-f32 and the GPU alike, in CoM-frame particle coordinates: no Jacobi vector is
//! formed here, nor anywhere in the step (the chart's Jacobi coordinates become particles once, at decode). Each
//! component is formed in this order, with every product that meets an addition written as an explicit `fma`, so a
//! backend that contracts a multiply and an add and one that does not round the same (gpu_determinism_note
//! § "The discipline", rules 2 and 6; whether every backend fuses an explicit `fma` is RQ-298's question, under its
//! "applied meanwhile"):
//!
//! ```text
//! S      = fma(m₂, x₂, fma(m₁, x₁, m₀·x₀))       the mass-weighted sum, body 0 first
//! R_com  = S / M
//! P_com  = (p₀ + p₁) + p₂
//! xᵢ     ← xᵢ − R_com                             each body, 0, 1, 2
//! pᵢ     ← fma(−wᵢ, P_com, pᵢ)                     wᵢ = mᵢ / M, cached with M
//! ```
//!
//! per component, `x` then `y`. The velocity form is the position form's: `V_com = S(v) / M`, `vᵢ ← vᵢ − V_com`.
//!
//! **What it costs.** About twenty operations for the projection itself. A projection moves the positions, so the
//! wrapper's cadence then re-forms the force evaluation at the projected positions, which the next step reads and the
//! monitor's energy reuses (`super::cadence`; the physics note on TASK-M3-04): the state the next step starts from is
//! then a function of the projected `(r, p)` alone.

use crate::Real;

/// The projection's constants, formed once from the masses (Part 1: `M` constant, computed once and cached): `M`
/// and the momentum weights `−mᵢ/M`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ComProjection<R> {
    /// The masses `m₀, m₁, m₂`, as the run's IC gave them.
    m: [R; 3],
    /// The total mass `M = (m₀ + m₁) + m₂`.
    total: R,
    /// `−mᵢ/M`, the weights the momentum projection subtracts `P_com` by.
    neg_weight: [R; 3],
}

impl<R: Real> ComProjection<R> {
    /// The projection for bodies of masses `m`: `M = (m₀ + m₁) + m₂` and `−mᵢ/M`, formed here and nowhere else.
    #[inline]
    pub fn new(m: &[R; 3]) -> Self {
        let total = (m[0] + m[1]) + m[2];
        ComProjection {
            m: *m,
            total,
            neg_weight: [-(m[0] / total), -(m[1] / total), -(m[2] / total)],
        }
    }

    /// The cached total mass `M`.
    #[inline]
    pub fn total_mass(&self) -> R {
        self.total
    }

    /// The masses the projection weights by, the ones [`ComProjection::new`] took.
    #[inline]
    pub fn masses(&self) -> [R; 3] {
        self.m
    }

    /// `Σ mᵢxᵢ / M` per component, the mass-weighted mean of `x` in the module's order: `R_com` of positions,
    /// `V_com` of velocities.
    #[inline]
    pub fn weighted_mean(&self, x: &[[R; 2]; 3]) -> [R; 2] {
        let m = &self.m;
        [
            m[2].fma(x[2][0], m[1].fma(x[1][0], m[0] * x[0][0])) / self.total,
            m[2].fma(x[2][1], m[1].fma(x[1][1], m[0] * x[0][1])) / self.total,
        ]
    }

    /// `(p₀ + p₁) + p₂` per component: `P_com`, the total momentum.
    #[inline]
    pub fn total_momentum(p: &[[R; 2]; 3]) -> [R; 2] {
        [(p[0][0] + p[1][0]) + p[2][0], (p[0][1] + p[1][1]) + p[2][1]]
    }

    /// `(r, p)` projected (the module's order): `rᵢ − R_com` and `pᵢ − (mᵢ/M)·P_com`, both sums formed from the
    /// state as it entered.
    #[inline]
    pub fn project(&self, r: &[[R; 2]; 3], p: &[[R; 2]; 3]) -> ([[R; 2]; 3], [[R; 2]; 3]) {
        let c = self.weighted_mean(r);
        let q = Self::total_momentum(p);
        let w = &self.neg_weight;
        (
            [
                [r[0][0] - c[0], r[0][1] - c[1]],
                [r[1][0] - c[0], r[1][1] - c[1]],
                [r[2][0] - c[0], r[2][1] - c[1]],
            ],
            [
                [w[0].fma(q[0], p[0][0]), w[0].fma(q[1], p[0][1])],
                [w[1].fma(q[0], p[1][0]), w[1].fma(q[1], p[1][1])],
                [w[2].fma(q[0], p[2][0]), w[2].fma(q[1], p[2][1])],
            ],
        )
    }

    /// The velocity form, for an occupant that evolves velocities: `vᵢ − V_com`, `V_com = Σ mᵢvᵢ / M`.
    #[inline]
    pub fn project_velocities(&self, v: &[[R; 2]; 3]) -> [[R; 2]; 3] {
        let c = self.weighted_mean(v);
        [
            [v[0][0] - c[0], v[0][1] - c[1]],
            [v[1][0] - c[0], v[1][1] - c[1]],
            [v[2][0] - c[0], v[2][1] - c[1]],
        ]
    }
}
