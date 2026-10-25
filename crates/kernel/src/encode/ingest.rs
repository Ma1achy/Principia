//! Ground-truth ingestion (inverse-encode contract Parts 4 and 7; validation ground-truth note § "New demands this
//! places on the inverse-encode contract"; REQ-VAL-014, REQ-ENC-025): a foreign physical state is canonicalised and
//! encoded, and the result exposes, beside `z`, the two diagnostics Part 7 asks for:
//!
//! - **the physical residual** `‖D(z) − C(x)‖_phys` ([`Ingested::residual`]), the bug signal: `D(z)`, the free decode
//!   of the `z` encode returned, measured from `C(x)`, the canonical representative of the input, in Part 4's norm,
//!   which the caller passes, as `super::by_kind::encode_into` takes it (REQ-ENC-029);
//! - **the conditioning number** `κ(z) = 1/σ_min(W·J_D(z))` ([`Ingested::kappa`], [`kappa`]), Part 4's definition: `J_D`
//!   the decode's 15 × 8 Jacobian at `z` (the three masses, positions and momenta against the eight controls), `W` the
//!   weight of `‖·‖_phys` at `D(z)` (1 on a mass, `1/R_x` on a position component, `1/P_x` on a momentum component).
//!
//! **How `κ` is evaluated** (Part 4, "How it is evaluated"): `J_D`'s column `k` by the central difference
//! `(D(z + hₖeₖ) − D(z − hₖeₖ))/(2hₖ)`, `hₖ = ε_R^{1/3}·max(1, |zₖ|)` at the precision `R`, each column weighted by `W`,
//! and `σ_min` by one-sided Jacobi rotations on the columns ([`sigma_min`]), so `σ_min²` is never formed. Where the
//! weighted Jacobian loses rank, `σ_min = 0` and `κ = +∞`.
//!
//! **CPU only.** Ingestion is the validation programme's import path (Part 7), run by `validation::import`; no shader
//! entry point reaches it, so it branches on floats where the GPU rules (gpu_determinism_note) would ask for selects.

use spirv_std::num_traits::Float;

use super::{encode, Encoded, SimplexClamp};
use crate::decode::{self, Ic, Latent};
use crate::generated::links::{LinkReal, Selection};
use crate::payload::PayloadReal;
use crate::real::IeeeBits;

/// A vector over the 15 physical components `D` produces: `m₀, m₁, m₂`, then `r₀, r₁, r₂` and `p₀, p₁, p₂`, each
/// `(x, y)`; a column of `W·J_D`.
pub type Phys<R> = [R; 15];

/// A foreign state ingested: what encode returned, the decode of its `z`, and Part 7's two diagnostics.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ingested<R> {
    /// `E(x)`: `z`, `C(x)` and the discard report.
    pub encoded: Encoded<R>,
    /// `D(z)`, the free decode of `encoded.z` under the same selection.
    pub decoded: Ic<R>,
    /// `‖D(z) − C(x)‖_phys`, `C(x)` the reference state (Part 7's output 2).
    pub residual: R,
    /// `κ(z)` (Part 4; Part 7's output 3).
    pub kappa: R,
}

/// Ingests `ic` under the selection `S`: `E(ic)`, `D(z)`, the residual `norm(C(ic), D(z))` and `κ(z)`. `norm(x, x′)`
/// is Part 4's `‖x′ − x‖_phys`, `x` the reference; `ic.m` is positive with `Σmᵢ = 1`, as `C` requires.
pub fn ingest<S, R, N>(ic: &Ic<R>, norm: N) -> Ingested<R>
where
    S: Selection,
    S::MassLink: SimplexClamp,
    R: LinkReal + PayloadReal + IeeeBits,
    N: Fn(&Ic<R>, &Ic<R>) -> R,
{
    let encoded = encode::<S, R>(ic);
    let decoded = decode::decode::<S, R>(&encoded.z).ic;
    Ingested {
        encoded,
        decoded,
        residual: norm(&encoded.canonical.ic, &decoded),
        kappa: kappa::<S, R>(&encoded.z),
    }
}

/// `x`'s 15 components, in [`Phys`]'s order.
pub fn components<R: Copy>(x: &Ic<R>) -> Phys<R> {
    [
        x.m[0], x.m[1], x.m[2], x.r[0][0], x.r[0][1], x.r[1][0], x.r[1][1], x.r[2][0], x.r[2][1],
        x.p[0][0], x.p[0][1], x.p[1][0], x.p[1][1], x.p[2][0], x.p[2][1],
    ]
}

/// `max(1, maxⱼ ‖vⱼ‖)`: Part 4's extent `R_x` or `P_x` of the block `v`.
fn extent<R: LinkReal>(v: &[[R; 2]; 3]) -> R {
    let mut e = R::one();
    for w in v {
        e = Float::max(e, Float::hypot(w[0], w[1]));
    }
    e
}

/// The weight of `‖·‖_phys` at `x` on each of the 15 components: 1 on a mass, `1/R_x` on a position component and
/// `1/P_x` on a momentum component (Part 4).
pub fn weights<R: LinkReal>(x: &Ic<R>) -> Phys<R> {
    let (r, p) = (R::one() / extent(&x.r), R::one() / extent(&x.p));
    let one = R::one();
    [one, one, one, r, r, r, r, r, r, p, p, p, p, p, p]
}

/// `W·J_D(z)`, column by column: column `k` is the central difference of the decode along `z[k]`, step
/// `hₖ = ε_R^{1/3}·max(1, |zₖ|)`, weighted by `W` at `D(z)`.
pub fn weighted_jacobian<S, R>(z: &Latent<R>) -> [Phys<R>; 8]
where
    S: Selection,
    R: LinkReal + PayloadReal,
{
    let w = weights(&decode::decode::<S, R>(z).ic);
    let step = Float::cbrt(<R as Float>::epsilon());
    let mut cols = [[R::zero(); 15]; 8];
    for (k, col) in cols.iter_mut().enumerate() {
        let h = step * Float::max(R::one(), Float::abs(z.z[k]));
        let (mut up, mut down) = (z.z, z.z);
        up[k] = up[k] + h;
        down[k] = down[k] - h;
        let plus = components(&decode::decode::<S, R>(&Latent::new(up)).ic);
        let minus = components(&decode::decode::<S, R>(&Latent::new(down)).ic);
        // The step actually taken: `z ± h` rounded, so the difference quotient divides by what was added.
        let span = up[k] - down[k];
        for (c, ((w, a), b)) in col.iter_mut().zip(w.iter().zip(&plus).zip(&minus)) {
            *c = *w * (*a - *b) / span;
        }
    }
    cols
}

/// `κ(z) = 1/σ_min(W·J_D(z))` under the selection `S` (Part 4), `+∞` where `σ_min = 0`.
pub fn kappa<S, R>(z: &Latent<R>) -> R
where
    S: Selection,
    R: LinkReal + PayloadReal,
{
    condition(weighted_jacobian::<S, R>(z))
}

/// `1/σ_min` of the 15 × 8 matrix whose columns are `cols`, `+∞` where `σ_min = 0`: [`kappa`] on `W·J_D(z)`.
pub fn condition<R: LinkReal>(cols: [Phys<R>; 8]) -> R {
    let s = sigma_min(cols);
    if s > R::zero() {
        R::one() / s
    } else {
        <R as Float>::infinity()
    }
}

/// `Σᵢ aᵢbᵢ` over the 15 components.
fn dot<R: LinkReal>(a: &Phys<R>, b: &Phys<R>) -> R {
    let mut s = R::zero();
    for (x, y) in a.iter().zip(b) {
        s = Float::mul_add(*x, *y, s);
    }
    s
}

/// The smallest singular value of the 15 × 8 matrix whose columns are `cols`, by one-sided Jacobi (Hestenes): pairs of
/// columns are rotated until every pair is orthogonal to the precision's epsilon, and the singular values are then the
/// columns' norms. A sweep visits every pair once; the sweeps stop when one rotates nothing, or after 64.
pub fn sigma_min<R: LinkReal>(mut cols: [Phys<R>; 8]) -> R {
    let eps = <R as Float>::epsilon();
    for _ in 0..64 {
        let mut rotated = false;
        for p in 0..8 {
            for q in (p + 1)..8 {
                let (a, b, g) = (
                    dot(&cols[p], &cols[p]),
                    dot(&cols[q], &cols[q]),
                    dot(&cols[p], &cols[q]),
                );
                if Float::abs(g) <= eps * Float::sqrt(a * b) {
                    continue;
                }
                rotated = true;
                let zeta = (b - a) / (g + g);
                let t = Float::signum(zeta) / (Float::abs(zeta) + Float::hypot(R::one(), zeta));
                let c = R::one() / Float::hypot(R::one(), t);
                let s = c * t;
                let (left, right) = cols.split_at_mut(q);
                for (u, v) in left[p].iter_mut().zip(right[0].iter_mut()) {
                    (*u, *v) = (c * *u - s * *v, s * *u + c * *v);
                }
            }
        }
        if !rotated {
            break;
        }
    }
    let mut least = Float::sqrt(dot(&cols[0], &cols[0]));
    for col in &cols[1..] {
        least = Float::min(least, Float::sqrt(dot(col, col)));
    }
    least
}
