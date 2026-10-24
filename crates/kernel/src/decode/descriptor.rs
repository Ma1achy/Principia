//! The `ICDescriptor`, what the IC *is* (dd_decoder §3.6; dd_generation_root §3.6; REQ-PAY-034): filled at the decode
//! stage, post-decode and pre-integration, from the decoded `(m, r, p)` alone, so every chart's IC is described alike
//! (dd_decoder §2: the canonical `(m, r, p)` is the one seam). Its twelve fields, in the generated struct's order:
//!
//! ```text
//! m0 m1 m2       the masses
//! q_mass         min(m₀, m₁)/max(m₀, m₁)
//! rho_mag        ‖ρ̃‖ = √μ_ρ·‖ρ‖,  ρ = r₁ − r₀,                       μ_ρ = m₀m₁/M₀₁
//! lambda_mag     ‖λ̃‖ = √μ_λ·‖λ‖,  λ = r₂ − (m₀r₀ + m₁r₁)/M₀₁,        μ_λ = m₂M₀₁
//! rho_ratio      ‖ρ̃‖/‖λ̃‖
//! rho_angle      atan2(ρ × λ, ρ · λ), radians on (−π, π]
//! K_0            Σᵢ ‖pᵢ‖²/2mᵢ
//! V_0            −Σ_{i<j} mᵢmⱼ/‖rᵢ − rⱼ‖                 (G = 1)
//! virial_ratio   2K₀/|V₀|
//! r_min_pair_0   min_{i<j} ‖rᵢ − rⱼ‖
//! ```
//!
//! The six fields after the masses are REQ-PAY-088's definitions, written into dd_decoder §3.6 by this task (R-72).
//! `E₀` is derived at read, `K_0 + V_0` ([`energy`]), never stored (R-86). The struct follows `Real` (R-313): the
//! f32 instantiation is the GPU's 64 B, the f64 the CPU's 112 B; `_pad` is declared padding, written zero.
//!
//! `2mᵢ` and `2K₀` are formed as the sums `mᵢ + mᵢ` and `K₀ + K₀`, which are exact at any `Real`.
//!
//! **The poles' values are selected, never divided** (R-436; dd_decoder §3.6, "Each producer selects its pole value").
//! The descriptor follows `Real`, so the GPU's decode stage forms it at f32, where WGSL lets an implementation assume
//! there are no infinities: a GPU-computed `1/0`, or arithmetic on `∞`, is not pinned to `±∞`. So each pole value is
//! chosen by comparing its divisor with 0 and a select ([`IeeeBits`]), the `±∞` written as its integer pattern's bits:
//! `rho_ratio` is `+∞` where `‖λ̃‖ = 0`; `V_0` is `−∞` wherever a pair separation is 0, for the whole sum, not a sum with
//! an infinite term ([`potential`]: a separation is 0 exactly where `r_min_pair_0`, their minimum, is); and
//! `virial_ratio` is 0 where `V_0` is non-finite, tested on its bits. The quotient a select discards is never read. The
//! CPU path runs this source, so it stores the same bits.
//!
//! **`rho_angle` on the axis** ([`angle`]; dd_decoder §3.6). On `ρ × λ = 0` the value is chosen, not `atan2`'s: the
//! image of π when `ρ · λ < 0`, whatever the sign of a zero `ρ × λ` (`atan2(−0, x < 0)` is −π), and 0 otherwise, the
//! pole `ρ × λ = ρ · λ = 0` included, where GLSL.std.450 `Atan2` is undefined. Off the axis `atan2`'s value is taken
//! down to the image of π, and one at or below its negative is read as the image, so the range is `(−π̂, π̂]`. Each
//! choice is a `select` on a comparison with a constant, between values already computed: no branch (the GPU
//! determinism note's discipline). `x + 0.0` is not used to clear a −0, since a shader compiler may fold it to `x`.

use spirv_std::num_traits::Float;

use crate::decode::Ic;
use crate::generated::links::LinkReal;
use crate::payload::{ICDescriptorOf, PayloadReal};
use crate::real::IeeeBits;

/// `a − b`.
#[inline]
fn sub<R: LinkReal>(a: [R; 2], b: [R; 2]) -> [R; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

/// `‖a‖ = √(a_x² + a_y²)`.
#[inline]
fn norm<R: LinkReal>(a: [R; 2]) -> R {
    Float::sqrt(a[0] * a[0] + a[1] * a[1])
}

/// The kinetic energy `K₀ = Σᵢ ‖pᵢ‖²/2mᵢ`.
#[inline]
pub fn kinetic<R: LinkReal>(ic: &Ic<R>) -> R {
    let term = |p: [R; 2], m: R| (p[0] * p[0] + p[1] * p[1]) / (m + m);
    term(ic.p[0], ic.m[0]) + term(ic.p[1], ic.m[1]) + term(ic.p[2], ic.m[2])
}

/// The pair separations `‖r₀ − r₁‖`, `‖r₀ − r₂‖`, `‖r₁ − r₂‖`.
#[inline]
fn separations<R: LinkReal>(r: [[R; 2]; 3]) -> [R; 3] {
    let [r0, r1, r2] = r;
    [norm(sub(r0, r1)), norm(sub(r0, r2)), norm(sub(r1, r2))]
}

/// The closest pair's separation, `min_{i<j} ‖rᵢ − rⱼ‖`, of the separations `d`.
#[inline]
fn closest<R: LinkReal>(d: [R; 3]) -> R {
    Float::min(Float::min(d[2], d[1]), d[0])
}

/// `V₀ = −Σ_{i<j} mᵢmⱼ/dᵢⱼ` of the masses `m` and the separations `d`, `−∞`'s bits where the closest, `d_min`, is 0
/// (R-436; the module's note).
#[inline]
fn potential_of<R: LinkReal + IeeeBits>(m: [R; 3], d: [R; 3], d_min: R) -> R {
    let [m0, m1, m2] = m;
    let sum = -(m0 * m1 / d[0] + m0 * m2 / d[1] + m1 * m2 / d[2]);
    sum.or_neg_inf(d_min == R::zero())
}

/// The potential energy `V₀ = −Σ_{i<j} mᵢmⱼ/‖rᵢ − rⱼ‖`, `G = 1`: `−∞`'s bits wherever a pair separation is 0, selected,
/// not divided (R-436).
#[inline]
pub fn potential<R: LinkReal + IeeeBits>(ic: &Ic<R>) -> R {
    let d = separations(ic.r);
    potential_of(ic.m, d, closest(d))
}

/// `virial_ratio = 2K₀/|V₀|` of `k = K₀` and `v = V₀`: 0 where `V₀` is non-finite, tested on its bits (R-436).
#[inline]
pub fn virial<R: LinkReal + IeeeBits>(k: R, v: R) -> R {
    select(v.non_finite_bits(), R::zero(), (k + k) / Float::abs(v))
}

/// `rho_ratio = ‖ρ̃‖/‖λ̃‖` of `rho_mag` and `lambda_mag`: `+∞`'s bits where `‖λ̃‖ = 0`, selected, not divided (R-436).
#[inline]
pub fn ratio<R: LinkReal + IeeeBits>(rho_mag: R, lambda_mag: R) -> R {
    (rho_mag / lambda_mag).or_pos_inf(lambda_mag == R::zero())
}

/// The derived `E₀ = K_0 + V_0`, formed at `Real` width from the stored fields (R-86; dd_generation_root §3.6).
#[inline]
pub fn energy<R: LinkReal + PayloadReal>(d: &ICDescriptorOf<R>) -> R {
    d.K_0 + d.V_0
}

/// `π̂`, dd_decoder §3.2's image of π: the β link's generated literal, π rounded toward zero at f32 and π's nearest
/// value at f64 (R-417), so `π̂ ≤ π` at each precision.
#[inline]
pub fn pi_image<R: LinkReal>() -> R {
    R::LIT_7
}

/// `t` where `c` holds, else `f`: both operands are computed before the choice, so it is a select, not a branch.
#[inline]
fn select<R: Copy>(c: bool, t: R, f: R) -> R {
    if c {
        t
    } else {
        f
    }
}

/// `rho_angle` from `ρ × λ` and `ρ · λ` (dd_decoder §3.6), on `(−π̂, π̂]`: on the axis `ρ × λ = 0`, `π̂` where
/// `ρ · λ < 0` and 0 otherwise, the pole included; elsewhere `atan2(cross, dot)` taken down to `π̂`, a value at or
/// below `−π̂` read as `π̂`, a zero `dot` taken as `+0` by a select on it against 0: Metal's `atan2` at its default
/// math mode gives `atan2(cross, −0)` the opposite sign of `π/2` to IEEE's (pitfalls §11). Off the axis IEEE's
/// `atan2(cross, ±0)` is one value, and on it the axis's select discards it, so the CPU's `rho_angle` is unchanged, bit
/// for bit.
#[inline]
pub fn angle<R: LinkReal>(cross: R, dot: R) -> R {
    let zero = R::zero();
    let pi = pi_image::<R>();
    let along = select(dot == zero, zero, dot);
    let off_axis = Float::min(Float::atan2(cross, along), pi);
    let off_axis = select(off_axis <= -pi, pi, off_axis);
    let on_axis = select(dot < zero, pi, zero);
    select(cross == zero, on_axis, off_axis)
}

/// The `ICDescriptor` of the decoded IC `ic` (dd_decoder §3.6).
#[inline]
pub fn describe<R: LinkReal + PayloadReal>(ic: &Ic<R>) -> ICDescriptorOf<R> {
    let [m0, m1, m2] = ic.m;
    let [r0, r1, r2] = ic.r;
    let m01 = m0 + m1;
    let rho = sub(r1, r0);
    let inner = [
        (m0 * r0[0] + m1 * r1[0]) / m01,
        (m0 * r0[1] + m1 * r1[1]) / m01,
    ];
    let lambda = sub(r2, inner);
    let rho_mag = Float::sqrt(m0 * m1 / m01) * norm(rho);
    let lambda_mag = Float::sqrt(m2 * m01) * norm(lambda);
    let cross = rho[0] * lambda[1] - rho[1] * lambda[0];
    let dot = rho[0] * lambda[0] + rho[1] * lambda[1];
    let k = kinetic(ic);
    let d = separations(ic.r);
    let r_min = closest(d);
    let v = potential_of(ic.m, d, r_min);
    ICDescriptorOf {
        m0,
        m1,
        m2,
        q_mass: Float::min(m0, m1) / Float::max(m0, m1),
        rho_mag,
        lambda_mag,
        rho_ratio: ratio(rho_mag, lambda_mag),
        rho_angle: angle(cross, dot),
        K_0: k,
        V_0: v,
        virial_ratio: virial(k, v),
        r_min_pair_0: r_min,
        _pad: Default::default(),
        _tail: Default::default(),
    }
}
