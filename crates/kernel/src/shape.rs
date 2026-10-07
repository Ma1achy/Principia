//! The live shape readout and the unwrapped phase (dd_integrator §3.7): the shape vector `n` derived from the state
//! each macro-step, never stored, and the running accumulator `θ̃`, the equatorial longitude `atan2(n_v, n_u)`
//! unwrapped by adding each step's principal-value delta. `orbit_count = ⌊|θ̃|/2π⌋` and `retrograde = θ̃ < 0` are
//! derived at read from `θ̃` (dd_simstate_payload §5; the read side's `orbit_count` and `retrograde`).
//!
//! **The start and the poles (R-389, R-392).** `θ̃` starts at 0 whatever `n(0)`'s longitude. Inside the pole disc,
//! polar radius `√(n_u² + n_v²) < r_pole`, the longitude is not trusted: `θ̃` holds, with the last longitude outside the
//! disc frozen as its reference, and on exit adds `wrap(exit longitude − reference)`. A difference of exactly ±π, per
//! step or on exit, adds +π: the wrap's interval is (−π, π]. An IC that starts inside the disc has no reference, so its
//! first exit adds nothing and `θ̃` counts from the exit longitude. `r_pole` is a calibration (R-71, REQ-INT-086), so
//! it is passed in, as a sim-key value, never a constant here.
//!
//! **The branch inputs** (R-34; gpu_determinism_note, "The discipline", rules 1, 2 and 6). Two floats decide a branch
//! here, each built in a fixed form so that every backend takes the same path:
//! - **The disc's squared radius**, [`DiscInput`]: `ρ²_I` against `r_pole²·I²`, `r_pole² = r_pole·r_pole` (one
//!   multiply), strict `<`, so a point at exactly `r_pole` is outside ([`in_disc`]). The test is homogeneous in the
//!   point, so the kernel decides it on the point scaled by `M₀₁ = m₀ + m₁`, built from the positions and masses by
//!   [`disc_input`] with no division and no `sqrt`: with `ρ = r₁ − r₀` and `Λ = M₀₁r₂ − (m₀r₀ + m₁r₁)`
//!   (`= M₀₁λ`), and `μ_ρμ_λ = m₀m₁m₂` (`M = 1`), `M₀₁u = U = m₀m₁‖ρ‖² − m₂‖Λ‖²`, `M₀₁v = 2√(m₀m₁m₂) ρ·Λ` and
//!   `M₀₁w = 2√(m₀m₁m₂) ρ×Λ`, so `ρ²_I = U² + 4m₀m₁m₂(ρ·Λ)²` and `I² = ρ²_I + 4m₀m₁m₂(ρ×Λ)²`. Every sum of
//!   products is an explicit `fma`, every other operation a lone one. [`theta_step_config`] steps on it; for a given
//!   shape point, [`point_disc_input`] is the point's own `(fma(v, v, u·u), fma(w, w, fma(v, v, u·u)))`, which
//!   [`theta_step`] uses.
//! - **`wrap`'s `d`**, in [`wrap`]: `longitude(n) − reference`, a difference of two `atan2` longitudes, against `π`
//!   (`d > π`) and `−π` (`d ≤ −π`). This is the one transcendental deciding a branch, and R-389 mandates it: θ̃ adds
//!   the principal value of the longitude difference, into (−π, π], exactly ±π giving +π (dd_integrator §3.7), and no
//!   form of that rule avoids a sign decision at the ±π tie. The longitude is [`shape`]'s normalised point's.
//!
//! Generic over the kernel's `Real`, f32 on the GPU and f64 natively (R-265). The float functions are called through
//! `Float`, as the read side calls them: the rust-gpu toolchain's `core` has no inherent `f32` math.

use spirv_std::num_traits::{Float, FloatConst};

use crate::Real;

/// The shape-sphere point `n = (u, v, w)/I` of the configuration `r` (each body's position) with masses `m`, by
/// Montgomery's map from the mass-weighted Jacobi vectors (dd_integrator §3.7; chart_reference §0.2, §3.1; R-14):
/// `ρ = r₁ − r₀` and `λ = r₂ − r₀₁`, `r₀₁` the pair's centre of mass, weighted as `ρ̃ = √μ_ρ ρ` and `λ̃ = √μ_λ λ`
/// with `μ_ρ = m₀m₁/M₀₁` and `μ_λ = m₂M₀₁` (`M = 1`, chart_reference §0.2); then `u = ‖ρ̃‖² − ‖λ̃‖²`, `v = 2 ρ̃·λ̃`,
/// `w = 2 ρ̃ ∧ λ̃` (the standard cross, R-14) and `I = ‖ρ̃‖² + ‖λ̃‖²`. The collision of bodies 0 and 1 is
/// `(−1, 0, 0)`; `L⁺`, the triangle 0 → 1 → 2 anticlockwise, is `(0, 0, 1)`.
#[inline]
pub fn shape<R: Real + Float>(r: [[R; 2]; 3], m: [R; 3]) -> [R; 3] {
    let two = R::one() + R::one();
    let m01 = m[0] + m[1];
    let mu_rho = m[0] * m[1] / m01;
    let mu_lambda = m[2] * m01;
    let rho = [r[1][0] - r[0][0], r[1][1] - r[0][1]];
    let r01 = [
        (m[0] * r[0][0] + m[1] * r[1][0]) / m01,
        (m[0] * r[0][1] + m[1] * r[1][1]) / m01,
    ];
    let lambda = [r[2][0] - r01[0], r[2][1] - r01[1]];
    let a = mu_rho * (rho[0] * rho[0] + rho[1] * rho[1]);
    let b = mu_lambda * (lambda[0] * lambda[0] + lambda[1] * lambda[1]);
    let s = Float::sqrt(mu_rho * mu_lambda);
    let p = s * (rho[0] * lambda[0] + rho[1] * lambda[1]);
    let q = s * (rho[0] * lambda[1] - rho[1] * lambda[0]);
    let i = a + b;
    [(a - b) / i, two * p / i, two * q / i]
}

/// The pole disc's branch input for a point: its squared polar radius `ρ²_I` and its squared norm `I²`, of the point
/// or of any positive multiple of it (the module's list).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DiscInput<R> {
    /// `ρ²_I = u² + v²`, scaled.
    pub rho2: R,
    /// `I² = u² + v² + w²`, scaled alike.
    pub i2: R,
}

/// The disc input of the configuration `r` with masses `m`, the shape point scaled by `M₀₁`, in its fixed form: no
/// division, no `sqrt`, every sum of products an explicit `fma` (the module's list; R-34).
#[inline]
pub fn disc_input<R: Real + Float>(r: [[R; 2]; 3], m: [R; 3]) -> DiscInput<R> {
    let two = R::one() + R::one();
    let m01 = m[0] + m[1];
    let rho = [r[1][0] - r[0][0], r[1][1] - r[0][1]];
    let big_lambda = [
        Float::mul_add(m01, r[2][0], -Float::mul_add(m[0], r[0][0], m[1] * r[1][0])),
        Float::mul_add(m01, r[2][1], -Float::mul_add(m[0], r[0][1], m[1] * r[1][1])),
    ];
    let m0m1 = m[0] * m[1];
    let four_m012 = two * two * (m0m1 * m[2]);
    let rr = Float::mul_add(rho[1], rho[1], rho[0] * rho[0]);
    let ll = Float::mul_add(big_lambda[1], big_lambda[1], big_lambda[0] * big_lambda[0]);
    let u = Float::mul_add(m0m1, rr, -(m[2] * ll));
    let dot = Float::mul_add(rho[1], big_lambda[1], rho[0] * big_lambda[0]);
    let cross = Float::mul_add(rho[0], big_lambda[1], -(rho[1] * big_lambda[0]));
    let rho2 = Float::mul_add(four_m012 * dot, dot, u * u);
    let i2 = Float::mul_add(four_m012 * cross, cross, rho2);
    DiscInput { rho2, i2 }
}

/// The disc input of the shape point `n = (u, v, w)`, or of any positive multiple of it: `ρ²_I = fma(v, v, u·u)` and
/// `I² = fma(w, w, ρ²_I)`, `‖(u, v, w)‖ = I` being the map's identity.
#[inline]
pub fn point_disc_input<R: Real + Float>(n: [R; 3]) -> DiscInput<R> {
    let rho2 = Float::mul_add(n[1], n[1], n[0] * n[0]);
    DiscInput {
        rho2,
        i2: Float::mul_add(n[2], n[2], rho2),
    }
}

/// Whether the point of disc input `d` lies inside the pole disc, polar radius below `r_pole` (R-389): its longitude
/// is not trusted there. `ρ²_I < r_pole·r_pole·I²`, strict: a point at exactly `r_pole` is outside.
#[inline]
pub fn in_disc<R: Real + Float>(d: DiscInput<R>, r_pole: R) -> bool {
    d.rho2 < r_pole * r_pole * d.i2
}

/// The equatorial longitude of `n`, `atan2(n_v, n_u)` (dd_integrator §3.7), in [−π, π].
#[inline]
pub fn longitude<R: Real + Float>(n: [R; 3]) -> R {
    Float::atan2(n[1], n[0])
}

/// Whether the shape point `n` lies inside the pole disc: [`in_disc`] on [`point_disc_input`].
#[inline]
pub fn in_pole_disc<R: Real + Float>(n: [R; 3], r_pole: R) -> bool {
    in_disc(point_disc_input(n), r_pole)
}

/// `d`, a difference of two longitudes in [−2π, 2π], wrapped into (−π, π]: the principal value, so exactly ±π gives
/// +π (R-389). `d` is a branch input, R-389's (see the module's list).
#[inline]
pub fn wrap<R: Real + Float + FloatConst>(d: R) -> R {
    let pi = R::PI();
    let tau = pi + pi;
    if d > pi {
        d - tau
    } else if d <= -pi {
        d + tau
    } else {
        d
    }
}

/// The unwrapped phase's running state: `θ̃` itself, and the longitude frozen on entering the pole disc, which a
/// step out of the disc differences against (R-389). `frozen` is meaningful only when `has_frozen`, which entering the
/// disc from outside sets; an IC that starts inside the disc has none (R-392).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theta<R> {
    /// `θ̃`, the net turning since the IC.
    pub theta: R,
    /// The last longitude outside the disc, stored on entering it.
    pub frozen: R,
    /// Whether `frozen` holds a longitude.
    pub has_frozen: bool,
}

impl<R: Real + Float> Theta<R> {
    /// `θ̃(0) = 0`, whatever `n(0)`'s longitude (R-389), with no frozen longitude. The first step's reference is
    /// `n(0)`'s longitude when `n(0)` lies outside the disc; when it lies inside, there is none (R-392).
    #[inline]
    pub fn start() -> Self {
        Theta {
            theta: R::zero(),
            frozen: R::zero(),
            has_frozen: false,
        }
    }
}

/// One macro-step of `θ̃`, from the shape point `n_prev` before the step to `n` after it, each point's side of the pole
/// disc decided on the point itself ([`in_pole_disc`]): for a given path of shape points. The kernel steps by
/// [`theta_step_config`].
#[inline]
pub fn theta_step<R: Real + Float + FloatConst>(
    theta: Theta<R>,
    n_prev: [R; 3],
    n: [R; 3],
    r_pole: R,
) -> Theta<R> {
    theta_step_held(
        theta,
        n_prev,
        n,
        in_pole_disc(n_prev, r_pole),
        in_pole_disc(n, r_pole),
    )
}

/// One macro-step of `θ̃` from the configuration `r_prev` to `r`, masses `m`: the longitudes [`shape`]'s, each side of
/// the pole disc decided on [`disc_input`], the fixed form (the module's list).
#[inline]
pub fn theta_step_config<R: Real + Float + FloatConst>(
    theta: Theta<R>,
    r_prev: [[R; 2]; 3],
    r: [[R; 2]; 3],
    m: [R; 3],
    r_pole: R,
) -> Theta<R> {
    theta_step_held(
        theta,
        shape(r_prev, m),
        shape(r, m),
        in_disc(disc_input(r_prev, m), r_pole),
        in_disc(disc_input(r, m), r_pole),
    )
}

/// One macro-step of `θ̃`, from the shape point `n_prev` before the step to `n` after it, with whether each lies inside
/// the pole disc decided (dd_integrator §3.7; R-389, R-392). The reference is the last trusted longitude: `n_prev`'s
/// when it lies outside the disc, otherwise the one frozen on entering.
/// - `n` inside the disc: no delta; on entering (`n_prev` outside) `n_prev`'s longitude is frozen, and while inside the
///   frozen one is kept.
/// - `n` outside: `wrap(longitude(n) − reference)` is added, the per-step principal-value delta, or on exit the wrapped
///   exit-minus-frozen difference; with no reference (an IC inside the disc, at its first exit) nothing is added, and
///   `θ̃` counts from this longitude on.
#[inline]
pub fn theta_step_held<R: Real + Float + FloatConst>(
    theta: Theta<R>,
    n_prev: [R; 3],
    n: [R; 3],
    prev_inside: bool,
    inside: bool,
) -> Theta<R> {
    let (reference, has_reference) = if prev_inside {
        (theta.frozen, theta.has_frozen)
    } else {
        (longitude(n_prev), true)
    };
    if inside {
        return Theta {
            theta: theta.theta,
            frozen: reference,
            has_frozen: has_reference,
        };
    }
    let delta = if has_reference {
        wrap(longitude(n) - reference)
    } else {
        R::zero()
    };
    Theta {
        theta: theta.theta + delta,
        ..theta
    }
}
