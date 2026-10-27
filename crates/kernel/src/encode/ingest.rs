//! Ground-truth ingestion (inverse-encode contract Parts 4 and 7; validation ground-truth note § "New demands this
//! places on the inverse-encode contract"; REQ-VAL-014, REQ-ENC-025): a foreign physical state is canonicalised and
//! encoded, and the result exposes, beside `z`, the two diagnostics Part 7 asks for:
//!
//! - **the physical residual** `‖D(z) − C(x)‖_phys` ([`Ingested::residual`]), the bug signal: `D(z)`, the free decode
//!   of the `z` encode returned, measured from `C(x)`, the canonical representative of the input, in Part 4's norm,
//!   which the caller passes, as `super::by_kind::encode_into` takes it (REQ-ENC-029);
//! - **the conditioning number** `κ(x) = ‖J_E(x)·W⁻¹‖₂` ([`Ingested::kappa`], [`kappa`]), Part 4's definition: `J_E`
//!   the encode's 8 × 15 Jacobian at the input `x` itself, on the canonical section or off it (the eight controls
//!   against the three masses, positions and momenta), `W` the weight of `‖·‖_phys` at `x` (1 on a mass, `1/R_x` on a
//!   position component, `1/P_x` on a momentum component): the ground-truth note's "`κ` of the encode Jacobian at the
//!   point".
//!
//! **How `κ` is evaluated** (Part 4, "How it is evaluated"): `J_E`'s column `k` by the central difference
//! `(E(x + hₖeₖ) − E(x − hₖeₖ))/(2hₖ)`, `hₖ = ε_R^{1/3}·max(1, |xₖ|)` at the precision `R`, each column divided by `W`'s
//! entry `wₖ` ([`weighted_encode_jacobian`]), and the largest singular value by one-sided Jacobi rotations on the
//! transpose's eight columns ([`sigma_max`]), so no Gram matrix is formed.
//!
//! **The section's tangent alone** ([`weighted_jacobian`], [`condition`]): `1/σ_min(W·J_D(z))` is the encode
//! Jacobian's norm on the section's tangent only. Since `J_E·J_D = I` there, it bounds `κ` from below; a state moved off
//! the section (its masses, centre of mass, total momentum, scale or orientation) can see more.
//!
//! **CPU only.** Ingestion is the validation programme's import path (Part 7), run by `validation::import`. The module
//! is gated off the SPIR-V build (`#[cfg(not(target_arch = "spirv"))]` in `encode/mod.rs`), as the driver's
//! `nsub_table` is: it uses `+∞` and a swept loop with `continue` and `break`, which the GPU rules (gpu_determinism_note
//! rules 4 and 5) ban in shader code.

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
    /// `κ(x)` at the input (Part 4; Part 7's output 3).
    pub kappa: R,
}

/// Ingests `ic` under the selection `S`: `E(ic)`, `D(z)`, the residual `norm(C(ic), D(z))` and `κ(ic)`. `norm(x, x′)`
/// is Part 4's `‖x′ − x‖_phys`, `x` the reference; `ic.m` is positive with `Σmᵢ = 1`, as `C` requires (`κ`'s mass
/// columns move one mass off the simplex: [`weighted_encode_jacobian`]).
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
        kappa: kappa::<S, R>(ic),
    }
}

/// `x`'s 15 components, in [`Phys`]'s order.
pub fn components<R: Copy>(x: &Ic<R>) -> Phys<R> {
    [
        x.m[0], x.m[1], x.m[2], x.r[0][0], x.r[0][1], x.r[1][0], x.r[1][1], x.r[2][0], x.r[2][1],
        x.p[0][0], x.p[0][1], x.p[1][0], x.p[1][1], x.p[2][0], x.p[2][1],
    ]
}

/// The state whose components are `c`, in [`Phys`]'s order.
pub fn state<R: Copy>(c: &Phys<R>) -> Ic<R> {
    Ic {
        m: [c[0], c[1], c[2]],
        r: [[c[3], c[4]], [c[5], c[6]], [c[7], c[8]]],
        p: [[c[9], c[10]], [c[11], c[12]], [c[13], c[14]]],
    }
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
/// `hₖ = ε_R^{1/3}·max(1, |zₖ|)`, weighted by `W` at `D(z)`. The section's tangent alone: [`condition`] of it bounds
/// [`kappa`] at `D(z)` from below (the module's note).
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

/// `(J_E(x)·W⁻¹)ᵀ`, as its eight columns, one per control: entry `k` of column `i` is the central difference of
/// `E(x)ᵢ` along `x`'s component `k`, step `hₖ = ε_R^{1/3}·max(1, |xₖ|)`, divided by `W`'s entry `wₖ` at `x`. `x` may be
/// on the canonical section or off it; every component is moved, the masses included.
///
/// **The mass columns** (Part 4, "The mass columns"): moving `mₖ` alone puts `Σmᵢ` at `1 ± hₖ`, the one use of
/// [`encode`] outside its precondition `Σmᵢ = 1`. They differentiate encode's own formulas off the simplex, the total
/// read as given: `C`'s centre of mass and total momentum with `M` taken as 1, and the mass block's inverse on the
/// masses as they stand. A mass within one step of zero (`mₖ ≤ hₖ`) takes the three-point forward difference
/// `(4E(x + hₖ) − 3E(x) − E(x + 2hₖ))/(2hₖ)`, so no mass reaches zero or below, where [`super::blocks`]' guard
/// substitutes the smallest normal and the difference would cross a kink.
pub fn weighted_encode_jacobian<S, R>(x: &Ic<R>) -> [Phys<R>; 8]
where
    S: Selection,
    S::MassLink: SimplexClamp,
    R: LinkReal + IeeeBits,
{
    let w = weights(x);
    let c = components(x);
    let step = Float::cbrt(<R as Float>::epsilon());
    let mut cols = [[R::zero(); 15]; 8];
    for (k, (ck, wk)) in c.iter().zip(&w).enumerate() {
        let h = step * Float::max(R::one(), Float::abs(*ck));
        let mut up = c;
        up[k] = *ck + h;
        let plus = encode::<S, R>(&state(&up)).z.z;
        if k < 3 && *ck <= h {
            // A mass within one step of zero: `(4E(x + h) − 3E(x) − E(x + 2h))/(2h)`, `2h` the step actually taken.
            let mut far = c;
            far[k] = *ck + (h + h);
            let beyond = encode::<S, R>(&state(&far)).z.z;
            let here = encode::<S, R>(x).z.z;
            let span = (far[k] - *ck) * *wk;
            let three = R::one() + R::one() + R::one();
            let four = three + R::one();
            for (col, ((a, b), e)) in cols.iter_mut().zip(plus.iter().zip(&beyond).zip(&here)) {
                col[k] = (four * *a - three * *e - *b) / span;
            }
        } else {
            let mut down = c;
            down[k] = *ck - h;
            let minus = encode::<S, R>(&state(&down)).z.z;
            // The step actually taken: `x ± h` rounded, so the difference quotient divides by what was added.
            let span = (up[k] - down[k]) * *wk;
            for (col, (a, b)) in cols.iter_mut().zip(plus.iter().zip(&minus)) {
                col[k] = (*a - *b) / span;
            }
        }
    }
    cols
}

/// `κ(x) = ‖J_E(x)·W⁻¹‖₂` under the selection `S` (Part 4): the largest singular value of the weighted encode
/// Jacobian at `x`, on the section or off it.
pub fn kappa<S, R>(x: &Ic<R>) -> R
where
    S: Selection,
    S::MassLink: SimplexClamp,
    R: LinkReal + IeeeBits,
{
    sigma_max(weighted_encode_jacobian::<S, R>(x))
}

/// `1/σ_min` of the 15 × 8 matrix whose columns are `cols`, `+∞` where `σ_min = 0`: on `W·J_D(z)`, the encode
/// Jacobian's norm on the section's tangent alone, a lower bound of [`kappa`] at `D(z)`.
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

/// The smallest singular value of the 15 × 8 matrix whose columns are `cols` ([`singular_values`]).
pub fn sigma_min<R: LinkReal>(cols: [Phys<R>; 8]) -> R {
    let v = singular_values(cols);
    let mut least = v[0];
    for s in &v[1..] {
        least = Float::min(least, *s);
    }
    least
}

/// The largest singular value of the 15 × 8 matrix whose columns are `cols` ([`singular_values`]): its operator
/// 2-norm, and its transpose's.
pub fn sigma_max<R: LinkReal>(cols: [Phys<R>; 8]) -> R {
    let v = singular_values(cols);
    let mut most = v[0];
    for s in &v[1..] {
        most = Float::max(most, *s);
    }
    most
}

/// The singular values of the 15 × 8 matrix whose columns are `cols`, unordered, by one-sided Jacobi (Hestenes): pairs
/// of columns are rotated until every pair is orthogonal to the precision's epsilon, and the singular values are then
/// the columns' norms. A sweep visits every pair once; the sweeps stop when one rotates nothing, or after 64.
pub fn singular_values<R: LinkReal>(mut cols: [Phys<R>; 8]) -> [R; 8] {
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
    cols.map(|col| Float::sqrt(dot(&col, &col)))
}
