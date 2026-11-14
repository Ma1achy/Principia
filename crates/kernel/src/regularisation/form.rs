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

/// A stepper's form on a regularised system: one per stepper of Part 2's table (integrator_contract Part 2b). Each
/// stepper type names its own as the associated constant [`crate::occupant::Stepper::FORM`], so the form a march takes
/// is fixed when the march is compiled for that stepper, never read from a profile at run time; a stepper that names
/// none fails to build under a regularisation.
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

    /// One step `h` of this form on `flow` from `(q, p)`. Under a regularisation `self` is a stepper's
    /// [`crate::occupant::Stepper::FORM`], a constant of the monomorphised march, so the match below selects one arm
    /// at compile time.
    #[inline(always)]
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
    rk4_stages(flow, q, p, h).next
}

/// A state `(q, p)` of a flow.
type Pair<F, R> = (<F as Flow<R>>::Q, <F as Flow<R>>::P);

/// RK4's stages on a flow of positions `Q` and momenta `P` ([`rk4_stages`]).
struct Rk4Stages<Q, P> {
    /// The stage states `y₁ … y₄`.
    y: [(Q, P); 4],
    /// Their rates `k₁ … k₄`.
    k: [(Q, P); 4],
    /// The state one step on.
    next: (Q, P),
}

/// The classical RK4 step's stages on the whole state: the stage states `y₁ = (q, p)`, `y₂ = y + (h/2)·k₁`,
/// `y₃ = y + (h/2)·k₂` and `y₄ = y + h·k₃`, their rates `kᵢ`, and the state one step on,
/// `y + (h/6)(k₁ + 2k₂ + 2k₃ + k₄)`. [`rk4`] is its last, so a joint step reading the stages ([`rk4_joint`]) leaves
/// the sample exactly the step's.
#[inline(always)]
fn rk4_stages<R: StepReal, F: Flow<R>>(
    flow: &F,
    q: &F::Q,
    p: &F::P,
    h: R,
) -> Rk4Stages<F::Q, F::P> {
    let one = R::one();
    let two = one + one;
    let half = h / two;
    let (k1q, k1p) = flow.rates(q, p);
    let y2 = (F::q_add(q, half, &k1q), F::p_add(p, half, &k1p));
    let (k2q, k2p) = flow.rates(&y2.0, &y2.1);
    let y3 = (F::q_add(q, half, &k2q), F::p_add(p, half, &k2p));
    let (k3q, k3p) = flow.rates(&y3.0, &y3.1);
    let y4 = (F::q_add(q, h, &k3q), F::p_add(p, h, &k3p));
    let (k4q, k4p) = flow.rates(&y4.0, &y4.1);
    let sixth = h / (two * (one + two));
    let next = (
        F::q_add(q, sixth, &F::q_weighted(two, [&k1q, &k2q, &k3q, &k4q])),
        F::p_add(p, sixth, &F::p_weighted(two, [&k1p, &k2p, &k3p, &k4p])),
    );
    Rk4Stages {
        y: [(*q, *p), y2, y3, y4],
        k: [(k1q, k1p), (k2q, k2p), (k3q, k3p), (k4q, k4p)],
        next,
    }
}

/// RK4's joint step on a regularised flow (R-974; dd_integrator §3.8, "Heggie × RK4's joint step"): the sample
/// `(q, p)` one step `h` on `flow`, [`Form::step`]'s under [`Form::Rk4`] bit for bit (the same `rk4_stages`), and
/// the displacement `(dq, dp)` through the same stages and the same tableau, each stage's rate replaced by the
/// difference `G(yᵢ + δyᵢ) − kᵢ`: `G` the rates of `shadow`, the flow the shadow follows, at the stage's state plus
/// the stage's displacement, each sum formed plainly at the working precision (R-974's (a1), the form TASK-M3-72's
/// measurement chose), less the sample's own stage rate `kᵢ`. Returns the sample's state, then the displacement's.
#[inline]
pub fn rk4_joint<R: StepReal, F: Flow<R>>(
    flow: &F,
    shadow: &F,
    q: &F::Q,
    p: &F::P,
    dq: &F::Q,
    dp: &F::P,
    h: R,
) -> (Pair<F, R>, Pair<F, R>) {
    let Rk4Stages {
        y: [y1, y2, y3, y4],
        k: [k1, k2, k3, k4],
        next,
    } = rk4_stages(flow, q, p, h);
    let one = R::one();
    let two = one + one;
    let half = h / two;
    // `G(y + δy) − k`: `y + 1·δy` is `y + δy` and `g + (−1)·k` is `g − k`, each one rounding per component.
    let diff = |y: &Pair<F, R>, eq: &F::Q, ep: &F::P, k: &Pair<F, R>| {
        let (gq, gp) = shadow.rates(&F::q_add(&y.0, one, eq), &F::p_add(&y.1, one, ep));
        (F::q_add(&gq, -one, &k.0), F::p_add(&gp, -one, &k.1))
    };
    let d1 = diff(&y1, dq, dp, &k1);
    let d2 = diff(
        &y2,
        &F::q_add(dq, half, &d1.0),
        &F::p_add(dp, half, &d1.1),
        &k2,
    );
    let d3 = diff(
        &y3,
        &F::q_add(dq, half, &d2.0),
        &F::p_add(dp, half, &d2.1),
        &k3,
    );
    let d4 = diff(&y4, &F::q_add(dq, h, &d3.0), &F::p_add(dp, h, &d3.1), &k4);
    let sixth = h / (two * (one + two));
    (
        next,
        (
            F::q_add(dq, sixth, &F::q_weighted(two, [&d1.0, &d2.0, &d3.0, &d4.0])),
            F::p_add(dp, sixth, &F::p_weighted(two, [&d1.1, &d2.1, &d3.1, &d4.1])),
        ),
    )
}
