//! Each stepper's exact form on a regularised (non-separable) system (integrator_contract Part 2b § "Every stepper ×
//! regularisation pair runs; the pairs measured not to work carry a warning (R-561)", "Every pair is defined exactly,
//! so its results reproduce"; R-561).
//!
//! Aarseth–Zare's `Γ` (and Heggie's `Γ*`) couples position and momentum, so a stepper is applied to the regularised
//! equations of motion `(u̇, ṫ) = ∂Γ/∂p`, `ṗ = −∂Γ/∂u` in fictitious time `τ`, at the step `h` the step control gives.
//! A [`Flow`] is such a system: its positions `Q` (the regularised positions with the time `t`), its momenta `P`, and
//! their rates. [`Form`] is each stepper's form on it, Part 2b's:
//!
//! - **KDK:** kick `p½ = p + (h/2)·ṗ(u, p)`; drift `(u₁, t₁) = (u, t) + h·(u̇, ṫ)(u, p½)`; kick
//!   `p₁ = p½ + (h/2)·ṗ(u₁, p½)`. Three evaluations of the flow, each at the state as it stands at that stage.
//! - **Yoshida-4 and Yoshida-6:** the KDK step above taken in sequence at `wᵢ·h`, dd_integrator §3.2's drift sequence
//!   (Yoshida-4: `w₁, w₀, w₁`; Yoshida-6: `w₃, w₂, w₁, w₀, w₁, w₂, w₃`), adjacent half-kicks not merged: 9 and 21
//!   evaluations per step.
//! - **RK4 and Euler:** their classical forms on the whole state `(u, p, t)`.
//!
//! **The pair's profile** ([`Form::pair_profile`]) is the pair's, not the stepper's row (Part 2b: "The explicit
//! composition is not symplectic on a non-separable Hamiltonian"): `symplectic` false for every pair, and
//! `step_reversible` false, dd test 3's measurement being the regularised occupants' (R-500) and none recorded for these
//! pairs; `run_reversible` false. RK4's row is the table's (integrator_contract Part 2a § "The table gains two rows":
//! order 4, 4 evaluations). The orders of the others are their forms' on a non-separable system: Euler's 1; KDK's 1,
//! since its drift reads `u̇` at the old `u` and so misses the `½h²·(∂u̇/∂u)·u̇` term of the exact flow, which a
//! separable Hamiltonian, where `u̇` does not depend on `u`, does not have; and a Yoshida composition of a first-order
//! step whose weights sum to one is first order too (Part 2b's note 2 measured Heggie × KDK at order 0.8–1.2). The
//! force evaluations are the flow evaluations per step: 1, 3, 9, 21 and 4.

use crate::occupant::{CapabilityProfile, StepReal};

/// A regularised system the steppers march: positions `Q` (the time among them), momenta `P`, and their rates.
pub trait Flow<R> {
    /// The positions, the time `t` among them.
    type Q: Copy;
    /// The momenta.
    type P: Copy;

    /// The rates of the positions, `(u̇, ṫ) = ∂Γ/∂p`, at `(q, p)`.
    fn q_rate(&self, q: &Self::Q, p: &Self::P) -> Self::Q;

    /// The rates of the momenta, `ṗ = −∂Γ/∂u`, at `(q, p)`.
    fn p_rate(&self, q: &Self::Q, p: &Self::P) -> Self::P;

    /// Both rates at `(q, p)`, one evaluation of the flow.
    #[inline]
    fn rates(&self, q: &Self::Q, p: &Self::P) -> (Self::Q, Self::P) {
        (self.q_rate(q, p), self.p_rate(q, p))
    }

    /// `q + h·dq`, by component.
    fn q_add(q: &Self::Q, h: R, dq: &Self::Q) -> Self::Q;

    /// `p + h·dp`, by component.
    fn p_add(p: &Self::P, h: R, dp: &Self::P) -> Self::P;

    /// `k₁ + 2k₂ + 2k₃ + k₄`, by component, `two` being `2`.
    fn q_weighted(two: R, k: [&Self::Q; 4]) -> Self::Q;

    /// `k₁ + 2k₂ + 2k₃ + k₄`, by component, `two` being `2`.
    fn p_weighted(two: R, k: [&Self::P; 4]) -> Self::P;
}

/// A stepper's form on a regularised system: one per stepper of Part 2's table (integrator_contract Part 2b).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    /// Explicit Euler on the whole state.
    Euler,
    /// The explicit kick–drift–kick.
    Kdk,
    /// Yoshida-4's sequence of KDK steps, `w₁, w₀, w₁`.
    Yoshida4,
    /// Yoshida-6's sequence of KDK steps, `w₃, w₂, w₁, w₀, w₁, w₂, w₃`.
    Yoshida6,
    /// The classical RK4 on the whole state.
    Rk4,
}

impl Form {
    /// The form of the stepper whose profile is `profile`, by its row of Part 2's table (dd_integrator §3.2): order 1
    /// with one evaluation, Euler; order 2 with one, KDK; order 4 with three, Yoshida-4; order 6 with seven, Yoshida-6;
    /// and RK4 otherwise, order 4 with four evaluations among them, the classical form a profile outside the table's
    /// five rows takes too.
    #[inline]
    pub const fn of(profile: &CapabilityProfile) -> Form {
        match (profile.order, profile.force_evals) {
            (1, 1) => Form::Euler,
            (2, 1) => Form::Kdk,
            (4, 3) => Form::Yoshida4,
            (6, 7) => Form::Yoshida6,
            _ => Form::Rk4,
        }
    }

    /// The profile of this stepper composed with a non-separable regularisation (the module's note).
    #[inline]
    pub const fn pair_profile(self) -> CapabilityProfile {
        let (order, force_evals) = match self {
            Form::Euler => (1, 1),
            Form::Kdk => (1, 3),
            Form::Yoshida4 => (1, 9),
            Form::Yoshida6 => (1, 21),
            Form::Rk4 => (4, 4),
        };
        CapabilityProfile {
            order,
            force_evals,
            symplectic: false,
            step_reversible: false,
            run_reversible: false,
            owns_time_mapping: true,
        }
    }

    /// One step `h` of this form on `flow` from `(q, p)`.
    #[inline]
    pub fn step<R: StepReal, F: Flow<R>>(self, flow: &F, q: &F::Q, p: &F::P, h: R) -> (F::Q, F::P) {
        match self {
            Form::Euler => euler(flow, q, p, h),
            Form::Kdk => kdk(flow, q, p, h),
            Form::Yoshida4 => {
                let (q, p) = kdk(flow, q, p, R::YOSHIDA4_W1 * h);
                let (q, p) = kdk(flow, &q, &p, R::YOSHIDA4_W0 * h);
                kdk(flow, &q, &p, R::YOSHIDA4_W1 * h)
            }
            Form::Yoshida6 => {
                let (q, p) = kdk(flow, q, p, R::YOSHIDA6_W3 * h);
                let (q, p) = kdk(flow, &q, &p, R::YOSHIDA6_W2 * h);
                let (q, p) = kdk(flow, &q, &p, R::YOSHIDA6_W1 * h);
                let (q, p) = kdk(flow, &q, &p, R::YOSHIDA6_W0 * h);
                let (q, p) = kdk(flow, &q, &p, R::YOSHIDA6_W1 * h);
                let (q, p) = kdk(flow, &q, &p, R::YOSHIDA6_W2 * h);
                kdk(flow, &q, &p, R::YOSHIDA6_W3 * h)
            }
            Form::Rk4 => rk4(flow, q, p, h),
        }
    }
}

/// Explicit Euler: `(q, p) + h·(q̇, ṗ)(q, p)`.
#[inline]
fn euler<R: StepReal, F: Flow<R>>(flow: &F, q: &F::Q, p: &F::P, h: R) -> (F::Q, F::P) {
    let (dq, dp) = flow.rates(q, p);
    (F::q_add(q, h, &dq), F::p_add(p, h, &dp))
}

/// The explicit kick–drift–kick (the module's note).
#[inline]
fn kdk<R: StepReal, F: Flow<R>>(flow: &F, q: &F::Q, p: &F::P, h: R) -> (F::Q, F::P) {
    let one = R::one();
    let half = h / (one + one);
    let p_half = F::p_add(p, half, &flow.p_rate(q, p));
    let q1 = F::q_add(q, h, &flow.q_rate(q, &p_half));
    let p1 = F::p_add(&p_half, half, &flow.p_rate(&q1, &p_half));
    (q1, p1)
}

/// The classical fourth-order Runge–Kutta step on the whole state.
#[inline]
fn rk4<R: StepReal, F: Flow<R>>(flow: &F, q: &F::Q, p: &F::P, h: R) -> (F::Q, F::P) {
    let one = R::one();
    let two = one + one;
    let half = h / two;
    let (k1q, k1p) = flow.rates(q, p);
    let (k2q, k2p) = flow.rates(&F::q_add(q, half, &k1q), &F::p_add(p, half, &k1p));
    let (k3q, k3p) = flow.rates(&F::q_add(q, half, &k2q), &F::p_add(p, half, &k2p));
    let (k4q, k4p) = flow.rates(&F::q_add(q, h, &k3q), &F::p_add(p, h, &k3p));
    let sixth = h / (two * (one + two));
    (
        F::q_add(q, sixth, &F::q_weighted(two, [&k1q, &k2q, &k3q, &k4q])),
        F::p_add(p, sixth, &F::p_weighted(two, [&k1p, &k2p, &k3p, &k4p])),
    )
}
