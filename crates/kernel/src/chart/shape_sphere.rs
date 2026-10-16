//! `ShapeSphere`, the shape-sphere chart (chart_reference §3, §5.1; chart_decoder_contract Part 1; R-14):
//! `Φ_S²(u, v) = (n(θ, φ), m_fixed, p_fixed)`, the masses and momenta held and only the shape varying.
//!
//! **The chart map** (§3.3; R-12, R-14, R-117), one of two, named by [`ShapeMap`]:
//! - [`ShapeMap::Spherical`], [`spherical`]: `θ = 2π·s` (the azimuth in the `(u, v)` plane, the horizontal axis),
//!   `φ = π·(1 − t)` (the polar angle from `+w`, the vertical axis, `L⁺` at the top) and
//!   `n = (sin φ cos θ, sin φ sin θ, cos φ)`, over the full `φ ∈ [0, π]` with no polar buffer (R-12). `(s, t)` is the
//!   chart's `(u, v)`, bottom-left origin and Y-up (REQ-SYS-009).
//! - [`ShapeMap::Exponential`], [`exponential`], the nonlinear chart: `n = cos(r)·n₀ + sin(r)·d/‖d‖` with
//!   `d = (2u − 1)·s·e₁ + (2v − 1)·s·e₂`, `r = ‖d‖`, about the orthonormal frame `(n₀, e₁, e₂)`; written
//!   `cos(r)·n₀ + (sin r/r)·d`, `sin r/r` selected as its limit 1 at `r = 0`, so `d = 0` gives `n₀` exactly.
//!
//! **The closed-form inverse** (§3.2; [`inverse`]): `a = I(1 + n₀)/2`, `b = I(1 − n₀)/2`, `p = I·n₁/2`,
//! `q = I·n₂/2`, `ρ̃ = √a·(cos φ_f, sin φ_f)`, `λ̃ = √b·(cos ψ, sin ψ)`, `ψ = φ_f + atan2(q, p)`, the standard-cross
//! sign (§3.1, R-14). At `n₀ = ±1`, `p = q = 0` and the `atan2` term is R-437's defined 0 for every sign of the zeros,
//! selected, never `atan2(0, 0)`, so `ψ = φ_f`. [`place`] then unweights and reconstructs per §0.2 through the shared
//! [`config::reconstruct`].
//!
//! **The smaller of `a` and `b`** is evaluated as `I(n₁² + n₂²)/(2(1 + |n₀|))`, equal to `I(1 − |n₀|)/2` on the unit
//! sphere, `1 − n₀² = n₁² + n₂²` (chart_reference §3.2; applied per R-369, TASK-M2-08); the larger is §3.2's own,
//! `I(1 + |n₀|)/2`. Near `n₀ = ±1`, `n₀ = cos r` rounds to `±1` once the polar radius `r` is below about `10⁻⁸`, so
//! `1 − |n₀|` loses `r` entirely while `n₁` and `n₂` keep it: as written, the round trip misses by up to `r` there,
//! about `10⁻⁸`. Through `n₁² + n₂²` it keeps `ab = p² + q²` to rounding, and both are non-negative for any `n`, with
//! the divisor at least 1.
//!
//! **The held values** (chart_reference §3, "The held values", R-72, REQ-CHART-048). `m_fixed` is the slice pin `z₀`'s
//! mass block through the shared `D_mass`, and `p_fixed` its momentum block through the shared `D_mom`, both under the
//! chart's link selection; `z₀`'s configuration block is not held, the shape replacing it. `p_fixed` is a
//! canonical-frame momentum, and `Φ` expresses it in the gauge of the configuration it builds, the inverse of the gauge
//! `C` then removes, decided as `C` decides it on that configuration ([`held_momentum`]): mirrored where `C` mirrors
//! ([`canon::mirror_decision`]), turned by `C`'s angle `φ = atan2(ρ_y, ρ_x)` ([`canon::rotation_angle`]) and scaled by
//! `I^{−1/4}`. So `C(Φ(u, v))` has masses `m_fixed` and momenta `p_fixed` at every pixel, to rounding, and the two
//! hemispheres' mirror points decode to one system (R-141). `I` and `φ_f` are gauge: they change `Φ`'s output, not
//! `C`'s. Their defaults are `I = 1` and `φ_f = 0`, at which `Φ` is already on `C`'s section on the upper hemisphere.
//!
//! **Into the shared decoder** (REQ-CHART-015). The chart sets the configuration explicitly, which `ChartOut`'s latent
//! controls cannot carry, so it overrides [`Chart::decode`]: the shared decoder's blocks run in its order, `D_mass` on
//! `z₀`'s mass block, [`place`] in place of `D_cfg`, `D_mom` on `z₀`'s momentum block placed by [`held_momentum`]; then `C` ([`canon::canonicalise`], its scale included) on the joined state, and
//! the shared descriptor of `C`'s output. So the decode's `ic` is the canonical `(m, r, p)`, `G = M = I = 1`, which
//! crosses to the integrator, and its descriptor that state's (dd_decoder §2, §3.6), while its `config` and `momentum`
//! stay the blocks' outputs in `Φ`'s gauge; `C` folds the lower hemisphere onto the upper at `(θ, π − φ)` (R-141).
//! `Φ` alone, without `C`, is [`place`] (R-178's round trip). [`Chart::map`] hands over the held blocks with the
//! configuration block 0: every `D ∘ Φ` entry, [`crate::chart::decode`] and [`crate::decode::energy_norm::decode_chart`],
//! goes through [`Chart::decode`], and the configuration block of `map`'s latent is never decoded.
//!
//! **The selects** (dd_decoder §2's select rule; gpu_determinism_note): three, each between values already computed and
//! continuous across its boundary, so a backend fork there moves the output by rounding only: `r = 0` in
//! [`exponential`] (`sin r/r → 1`), `n₀ ≥ 0` in [`inverse`] (both forms give `I/2` at `n₀ = 0`), and R-437's
//! `p = q = 0` in [`inverse`] (the defined 0).
//!
//! **Validation** (inverse_encode_contract § "Chart-aware validation"; R-26). Layer 2's `θ ∈ [0, 2π)` and `φ ∈ [0, π]`
//! are the point's own azimuth and polar angle, which every point of S² has: past `[0,1]²` the spherical map's θ is
//! periodic and its φ pole-crossing, so its pixels there are systems `[0,1]²` holds (chart_reference §5.4), and the
//! exponential map's are points of S² as well, its bounded axes hatched by their declaration, not by `validate`.
//! Layer 3 holds by construction: the masses are `D_mass`'s, the CoM is at the origin by §0.2's reconstruction, and
//! `Σpᵢ = 0` by `D_mom`, kept by a mirror, a turn and a scale. So `validate` is `Valid` at every `(u, v)`.
//!
//! **π** is taken at the parameters' type, `P::PI()` widened (`F: From<P>`), as every parameter is: the trait's float
//! bound has no constant of its own.

use spirv_std::num_traits::{Float, FloatConst};

use super::{Chart, ChartOut, ValidationResult};
use crate::canon;
use crate::decode::config::{self, Config};
use crate::decode::mass::{self, Masses};
use crate::decode::momentum::{self, Momentum};
use crate::decode::{self, descriptor, Decoded, Ic};
use crate::generated::links::{LinkReal, Selection};
use crate::payload::PayloadReal;

/// Which chart map the shape sphere uses (chart_reference §3.3: "state which is used").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapeMap {
    /// The spherical map, `θ = 2π·s`, `φ = π·(1 − t)` ([`spherical`]).
    Spherical,
    /// The exponential map about `(n₀, e₁, e₂)` ([`exponential`]), the nonlinear chart.
    Exponential,
}

/// The shape-sphere chart: its map, the exponential map's frame and extent, the gauge `I` and `φ_f`, and the held
/// blocks of the slice pin `z₀` (the module's notes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShapeSphere<P> {
    /// The chart map.
    pub map: ShapeMap,
    /// The exponential map's centre `n₀`, a unit vector.
    pub n0: [P; 3],
    /// The exponential map's horizontal direction `e₁`, orthonormal to `n₀` and `e₂`.
    pub e1: [P; 3],
    /// The exponential map's vertical direction `e₂`.
    pub e2: [P; 3],
    /// The exponential map's extent `s`, the angle from `n₀` to the middle of an edge of `[0,1]²`.
    pub scale: P,
    /// The moment of inertia `I > 0` of `Φ`'s configuration, `‖ρ̃‖² + ‖λ̃‖²`; 1 by default.
    pub inertia: P,
    /// The fibre phase `φ_f`, `ρ̃`'s angle from `+x`; 0 by default.
    pub phase: P,
    /// `z₀`'s mass block `(z_μ1, z_μ2)`, giving `m_fixed`.
    pub z_mass: [P; 2],
    /// `z₀`'s momentum block `(z_q0, …, z_q3)`, giving `p_fixed`.
    pub z_mom: [P; 4],
}

/// The spherical map at `(s, t)`, with `pi` the float type's π: `θ = 2π·s`, `φ = π·(1 − t)`,
/// `n = (sin φ cos θ, sin φ sin θ, cos φ)` (chart_reference §3.3, R-14).
#[inline]
pub fn spherical<F: Float>(s: F, t: F, pi: F) -> [F; 3] {
    let theta = (pi + pi) * s;
    let phi = pi * (F::one() - t);
    let sin_phi = Float::sin(phi);
    [
        sin_phi * Float::cos(theta),
        sin_phi * Float::sin(theta),
        Float::cos(phi),
    ]
}

/// The exponential map at `(u, v)` about the frame `(n0, e1, e2)` with extent `scale`: `d = (2u − 1)·s·e₁ +
/// (2v − 1)·s·e₂`, `r = ‖d‖`, `n = cos(r)·n₀ + (sin r/r)·d`, `sin r/r` selected as 1 at `r = 0` (chart_reference §3.3).
#[inline]
pub fn exponential<F: Float>(n0: [F; 3], e1: [F; 3], e2: [F; 3], scale: F, u: F, v: F) -> [F; 3] {
    let a = (u + u - F::one()) * scale;
    let b = (v + v - F::one()) * scale;
    let d = [
        a * e1[0] + b * e2[0],
        a * e1[1] + b * e2[1],
        a * e1[2] + b * e2[2],
    ];
    let r = Float::sqrt(d[0] * d[0] + d[1] * d[1] + d[2] * d[2]);
    let c = Float::cos(r);
    let k = select(r == F::zero(), F::one(), Float::sin(r) / r);
    [
        c * n0[0] + k * d[0],
        c * n0[1] + k * d[1],
        c * n0[2] + k * d[2],
    ]
}

/// What [`inverse`] gives: the mass-weighted Jacobi vectors and the shape's hyperspherical angles in `Φ`'s frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Inverse<R> {
    /// `ρ̃ = √a·(cos φ_f, sin φ_f)`.
    pub rho_t: [R; 2],
    /// `λ̃ = √b·(cos ψ, sin ψ)`.
    pub lambda_t: [R; 2],
    /// `α = atan2(√b, √a)`, so `‖ρ̃‖ = √I·cos α`, as the canonical-frame decode's α; not a pole, `a + b = I > 0`.
    pub alpha: R,
    /// `β = ψ − φ_f`, the `atan2(q, p)` term: `λ̃`'s angle from `ρ̃`, in `(−π, π]`, and R-437's 0 at `n₀ = ±1`.
    pub beta: R,
}

/// The closed-form inverse of chart_reference §3.2 at the point `n` (a unit vector), the moment of inertia `inertia`
/// and the fibre phase `phase`, the smaller of `a` and `b` through `n₁² + n₂²` (the module's notes; R-437 at the
/// poles).
#[inline]
pub fn inverse<R: Float>(n: [R; 3], inertia: R, phase: R) -> Inverse<R> {
    let one = R::one();
    let two = one + one;
    let pole = one + Float::abs(n[0]);
    let large = inertia * pole / two;
    let small = inertia * (n[1] * n[1] + n[2] * n[2]) / (two * pole);
    let upper = n[0] >= R::zero();
    let a = select(upper, large, small);
    let b = select(upper, small, large);
    let p = inertia * n[1] / two;
    let q = inertia * n[2] / two;
    let beta = select(
        (p == R::zero()) & (q == R::zero()),
        R::zero(),
        Float::atan2(q, p),
    );
    let psi = phase + beta;
    let (root_a, root_b) = (Float::sqrt(a), Float::sqrt(b));
    Inverse {
        rho_t: [root_a * Float::cos(phase), root_a * Float::sin(phase)],
        lambda_t: [root_b * Float::cos(psi), root_b * Float::sin(psi)],
        alpha: Float::atan2(root_b, root_a),
        beta,
    }
}

/// `Φ`'s configuration at the point `n` under `masses`: [`inverse`], then unweighted and reconstructed per §0.2
/// through the shared [`config::reconstruct`]. Its `alpha` and `beta` are [`Inverse`]'s.
#[inline]
pub fn place<R: LinkReal>(n: [R; 3], inertia: R, phase: R, masses: &Masses<R>) -> Config<R> {
    let inv = inverse(n, inertia, phase);
    let (rho, lambda, r) = config::reconstruct(masses, inv.rho_t, inv.lambda_t);
    Config {
        alpha: inv.alpha,
        beta: inv.beta,
        rho_t: inv.rho_t,
        lambda_t: inv.lambda_t,
        rho,
        lambda,
        r,
    }
}

/// `p_fixed`, `D_mom` on `z_mom` under `S`, in the gauge of `config` (the module's notes): each Jacobi momentum
/// mirrored where `C` mirrors `config`, turned by `C`'s angle on it and scaled by `I^{−1/4}`, and the particle momenta
/// rebuilt from them by the shared [`momentum::to_particles`]. The controls `q` are `D_mom`'s.
#[inline]
pub fn held_momentum<S: Selection, R: LinkReal + PayloadReal>(
    z_mom: [R; 4],
    masses: &Masses<R>,
    config: &Config<R>,
    inertia: R,
) -> Momentum<R> {
    let fixed = momentum::decode::<S, R>(z_mom, masses);
    let r = config.r;
    let phi = canon::rotation_angle([r[1][0] - r[0][0], r[1][1] - r[0][1]]);
    let flip = canon::mirror_decision(masses.m, r);
    let (s, c) = (Float::sin(phi), Float::cos(phi));
    let k = R::one() / Float::sqrt(Float::sqrt(inertia));
    let gauge = |v: [R; 2]| {
        let y = select(flip, -v[1], v[1]);
        [k * (c * v[0] - s * y), k * (s * v[0] + c * y)]
    };
    let p_rho = gauge(fixed.p_rho);
    let p_lambda = gauge(fixed.p_lambda);
    Momentum {
        q: fixed.q,
        p_rho,
        p_lambda,
        p: momentum::to_particles(masses, p_rho, p_lambda),
    }
}

impl<P: Copy + FloatConst> ShapeSphere<P> {
    /// The chart's point `n` at `(u, v)` under its map, at the float type `F`.
    #[inline]
    pub fn point<F: Float + From<P>>(&self, u: F, v: F) -> [F; 3] {
        let wide = <F as From<P>>::from;
        match self.map {
            ShapeMap::Spherical => spherical(u, v, wide(P::PI())),
            ShapeMap::Exponential => exponential(
                self.n0.map(wide),
                self.e1.map(wide),
                self.e2.map(wide),
                wide(self.scale),
                u,
                v,
            ),
        }
    }
}

impl<P: Copy + FloatConst> Chart for ShapeSphere<P> {
    type Param = P;

    /// The held blocks, `z₀`'s momentum and mass blocks, in the decoder's layout, with the configuration block 0: the
    /// chart sets the configuration explicitly, and every `D ∘ Φ` entry decodes it through [`Chart::decode`].
    #[inline]
    fn map<F: Float + From<P>>(&self, _u: F, _v: F) -> ChartOut<F> {
        let wide = <F as From<P>>::from;
        ChartOut {
            z: decode::Latent::new([
                F::zero(),
                F::zero(),
                wide(self.z_mom[0]),
                wide(self.z_mom[1]),
                wide(self.z_mom[2]),
                wide(self.z_mom[3]),
                wide(self.z_mass[0]),
                wide(self.z_mass[1]),
            ]),
        }
    }

    /// `D ∘ Φ` with the configuration set explicitly (the module's notes): mass → [`place`] → [`held_momentum`], then
    /// `C` on the joined state and the descriptor of `C`'s output. The `ic` is canonical;
    /// `config` and `momentum` are the blocks' outputs, in `Φ`'s gauge.
    #[inline]
    fn decode<S, R>(&self, u: R, v: R) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        let wide = <R as From<P>>::from;
        let inertia = wide(self.inertia);
        let masses = mass::decode::<S, R>(self.z_mass.map(wide));
        let config = place(self.point(u, v), inertia, wide(self.phase), &masses);
        let momentum = held_momentum::<S, R>(self.z_mom.map(wide), &masses, &config, inertia);
        let ic = canon::canonicalise(&Ic {
            m: masses.m,
            r: config.r,
            p: momentum.p,
        })
        .ic;
        Decoded {
            ic,
            masses,
            config,
            momentum,
            descriptor: descriptor::describe(&ic),
            degenerate: masses.degenerate,
        }
    }

    fn name(&self) -> &str {
        "ShapeSphere"
    }

    /// [`ValidationResult::Valid`] at every `(u, v)` (the module's notes).
    fn validate(&self, _u: f64, _v: f64) -> ValidationResult {
        ValidationResult::Valid
    }
}

/// `t` where `c` holds, else `f`: a select, both operands computed.
#[inline]
fn select<T: Copy>(c: bool, t: T, f: T) -> T {
    if c {
        t
    } else {
        f
    }
}
