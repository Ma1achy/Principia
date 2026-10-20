//! `MassSimplex { nu0, eps_m }`, the ternary mass plot (chart_reference §4.4's mass-simplex relaxation, §4.5;
//! REQ-CHART-026): the geometry fixed at `ν₀`, the masses swept over the simplex, the bodies at rest.
//!
//! **The map** (§4.5, 0-based, R-22). `(u, v)` goes to the simplex by the shear `x = u`, `y = (1 − u)·v`, with
//! `m₀ = 1 − x − y`, `m₁ = x`, `m₂ = y`, then toward the barycentre by the interior buffer `ε_m` ([`masses`]):
//! `m ← (1 − 3ε_m)·m + ε_m·(1, 1, 1)`, whose default `10⁻⁴` is the register's `mass_simplex_eps_m`. In `[0,1]²` the raw
//! masses are `≥ 0` at either precision (`1 − x − y` is formed as `(1 − u) − (1 − u)·v`, a float minus at most itself),
//! so each buffered mass is `≥ ε_m` and they sum to 1 to rounding. `Φ_mass(u, v) = (ν₀, (m₀, m₁, m₂), rest)`.
//!
//! **The fixed geometry** ([`system`]). The triangle is the Burrau triangle at `ν₀` (chart_reference §4.2, §4.3: `a =
//! 1 − ν₀²`, `b = 2ν₀`, `c = 1 + ν₀²`): `r₀ = (0, 0)`, `r₁ = (a/c, 0)`, `r₂ = (0, b/c)`, the positions held while the
//! masses move. So the Jacobi vectors are those of these positions under the pixel's masses: `ρ = r₁ − r₀ = (a/c, 0)`,
//! `λ = r₂ − (m₀r₀ + m₁r₁)/M₀₁ = (−(m₁/M₀₁)·a/c, b/c)`, `M₀₁` the mass block's one rounded sum. At the Burrau masses
//! `m₁/M₀₁ = b/(b + c)` and `λ` is §4.2's. The decode is the Burrau family's at rest ([`RestBlocks`]): the masses through
//! the shared entry for a chart that sets masses ([`crate::decode::mass::from_masses`], whose `M01_TINY` test applies;
//! its floor here is `2ε_m`, above `ε_M01`), the configuration the shared canonical-frame decode of an explicit `(ρ, λ)`
//! (`ρ` on `+x`, `λ_y = b/c > 0`, so `C` is a no-op), every momentum 0, and the shared descriptor. `Φ`'s
//! [`ChartOut`] carries the zero vector, which none of these blocks reads.
//!
//! **The Burrau marker** ([`MassSimplex::marker`]): the overlay point where `Φ_mass` gives the Burrau masses
//! `(c, b, a)/(a + b + c)` at `ν₀`, the buffer undone (`m = (m′ − ε_m)/(1 − 3ε_m)`) and the shear inverted
//! (`u = m₁`, `v = m₂/(1 − m₁)`). A display coordinate, CPU-side, at f64.
//!
//! **Per-pixel mass** (inverse_encode_contract § "Chart-aware validation", the chart flags; lowering Part 3): the
//! masses vary with `(u, v)`, so the chart declares `requires_per_pixel_mass`, and its masses come only from its own
//! decode.
//!
//! **Validation** (R-26; inverse_encode_contract layers 2 and 3). Layer 2 is the mass simplex narrowed by the buffer,
//! buffered `mᵢ ≥ ε_m`, which is exactly raw `mᵢ ≥ 0`; it is decided on the raw masses' signs, exactly (`m₁ = u`,
//! `m₂ = (1 − u)·v`, `m₀ = (1 − u)·(1 − v)`, each sign that of a product of two exactly signed differences), so the
//! feasible set is `[0,1]²` and the edge `u = 1`, and nothing past `[0,1]²` is let through by rounding the buffer. A
//! point outside it is clamped onto `[0,1]²`, each out-of-range coordinate snapped to its edge
//! ([`Constraint::MassBuffer`]): the edge's buffered masses are an IC of the same kind, near-zero masses held at `ε_m`.
//! A non-finite coordinate is rejected. The geometry needs `ν₀ ∈ (0, 1)` ([`Constraint::BurrauNu`]: outside it a side is
//! zero or negative). Layer 3, decode sanity, holds by construction at every point layer 2 admits: the masses are
//! `≥ ε_m > 0`; the positions are finite, `ρ = (a/c, 0)` with `a = 1 − ν₀² > 0` and `μ_ρ`, `μ_λ` bounded below by the
//! buffer, so the scale gauge divides by no zero, and the reconstruction puts the CoM at 0; every momentum is 0.

use spirv_std::num_traits::Float;

use super::burrau::{embed, RestBlocks};
use super::invariant::WarpReal;
use super::{Chart, ChartDecode, ChartOut, Constraint, ValidationResult};
use crate::burrau::System;
use crate::decode::{self, Decoded, Latent};
use crate::generated::links::{LinkReal, Selection};
use crate::payload::PayloadReal;

/// The ternary mass plot (chart_reference §4.5): the triangle fixed at `nu0`, the masses on the simplex buffered by
/// `eps_m`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MassSimplex<P> {
    /// `ν₀ ∈ (0, 1)`, the fixed geometry's shape: the lock's, or lookup's partial-spec default `ν = 1/2` with no lock.
    pub nu0: P,
    /// `ε_m`, the interior buffer, `0 < ε_m < 1/3`; the register's `mass_simplex_eps_m` by default.
    pub eps_m: P,
}

/// The buffered masses at `(u, v)` (the module's note): the shear `x = u`, `y = (1 − u)·v`, `m₀ = (1 − u) − y`, then
/// `(1 − 3ε)·m + ε` componentwise.
#[inline]
pub fn masses<F: Float>(eps: F, u: F, v: F) -> [F; 3] {
    let rest = F::one() - u;
    let y = rest * v;
    let k = F::one() - (eps + eps + eps);
    [k * (rest - y) + eps, k * u + eps, k * y + eps]
}

/// The system at the masses `m` on the Burrau triangle at `nu0` (the module's note): its positions, `ρ = (a/c, 0)` and
/// `λ = (−(m₁/M₀₁)·a/c, b/c)`, `M₀₁` formed as the mass block forms it.
#[inline]
pub fn system<R: Float>(nu0: R, m: [R; 3]) -> System<R> {
    let triangle = embed(nu0);
    let a_c = triangle.rho[0];
    let m01 = Float::mul_add(m[1], R::one(), m[0]);
    System {
        m,
        r: triangle.r,
        rho: triangle.rho,
        lambda: [-(m[1] / m01 * a_c), triangle.lambda[1]],
    }
}

impl<P: Copy> MassSimplex<P> {
    /// The buffered masses at `(u, v)`, [`masses`] under the chart's `ε_m`.
    #[inline]
    pub fn masses<F: Float + From<P>>(&self, u: F, v: F) -> [F; 3] {
        masses(<F as From<P>>::from(self.eps_m), u, v)
    }
}

impl<P: Copy> MassSimplex<P>
where
    f64: From<P>,
{
    /// The Burrau marker's `(u, v)` (the module's note): where `Φ_mass` gives the Burrau masses at `ν₀`.
    pub fn marker(&self) -> [f64; 2] {
        let eps = f64::from(self.eps_m);
        let burrau = embed(f64::from(self.nu0)).m;
        let raw = |m: f64| (m - eps) / (1.0 - 3.0 * eps);
        let (m1, m2) = (raw(burrau[1]), raw(burrau[2]));
        [m1, m2 / (1.0 - m1)]
    }
}

/// `D ∘ Φ_mass` at `(u, v)`: the system at the buffered masses on the triangle at `ν₀`, at rest.
#[inline]
pub fn decode<R, P>(chart: &MassSimplex<P>, u: R, v: R) -> Decoded<R>
where
    R: LinkReal + PayloadReal + From<P>,
    P: Copy,
{
    let system = system(<R as From<P>>::from(chart.nu0), chart.masses(u, v));
    decode::decode_with(&mut RestBlocks::new(system), &Latent::new([R::zero(); 8]))
}

/// Whether `a·b ≥ 0`, decided on the factors' signs, so no underflow of the product decides it.
#[inline]
fn product_nonnegative(a: f64, b: f64) -> bool {
    (a >= 0.0 && b >= 0.0) || (a <= 0.0 && b <= 0.0)
}

/// Layer 2 at `(u, v)`: every raw mass `≥ 0`, which is every buffered mass `≥ ε_m` (the module's note).
#[inline]
fn in_simplex(u: f64, v: f64) -> bool {
    let rest = 1.0 - u;
    u >= 0.0 && product_nonnegative(rest, v) && product_nonnegative(rest, 1.0 - v)
}

impl<P: Copy> Chart for MassSimplex<P>
where
    f64: From<P>,
{
    type Param = P;

    /// The zero vector, unread (the module's note).
    #[inline]
    fn map<F: Float + From<P>>(&self, _u: F, _v: F) -> ChartOut<F> {
        ChartOut {
            z: Latent::new([F::zero(); 8]),
        }
    }

    fn name(&self) -> &str {
        "MassSimplex"
    }

    /// `true`: the masses are the pixel's (the module's note).
    fn requires_per_pixel_mass(&self) -> bool {
        true
    }

    /// [`decode()`]: the chart's masses on its fixed triangle, at rest.
    #[inline]
    fn decode<S, R>(&self, u: R, v: R) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        decode(self, u, v)
    }

    /// `ν₀ ∈ (0, 1)` and layer 2's buffered simplex; layer 3 holds by construction (the module's note).
    fn validate(&self, u: f64, v: f64) -> ValidationResult {
        let nu0 = f64::from(self.nu0);
        if !(nu0 > 0.0 && nu0 < 1.0) {
            return ValidationResult::Reject {
                failed: Constraint::BurrauNu,
            };
        }
        if !(u.is_finite() && v.is_finite()) {
            return ValidationResult::Reject {
                failed: Constraint::MassBuffer,
            };
        }
        if in_simplex(u, v) {
            return ValidationResult::Valid;
        }
        let (cu, cv) = (u.clamp(0.0, 1.0), v.clamp(0.0, 1.0));
        ValidationResult::Clamp {
            u: cu,
            v: cv,
            moved: [cu != u, cv != v],
            failed: Constraint::MassBuffer,
        }
    }
}

impl<P: Copy> ChartDecode for MassSimplex<P>
where
    f64: From<P>,
{
    /// The chart's own [`Chart::decode`], [`decode()`].
    #[inline]
    fn decode_at<S, R>(&self, u: R, v: R) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + WarpReal + From<P>,
    {
        decode(self, u, v)
    }
}
