//! The Burrau-family charts (chart_reference §4.3, §4.5; REQ-CHART-022, REQ-CHART-023, REQ-CHART-013): the continuous
//! `ν` axis, its embed `γ : ν → (configuration, masses)`, the `(ν, K)` chart ([`Burrau`]) and the Euclid plane
//! ([`EuclidPlane`]). Each is an ordinary chart map into the shared decoder: no Burrau-specific integrator or renderer
//! (§4.5).
//!
//! **The `ν` axis** ([`embed`]; §4.3, §4.5). Euclid's formulae hold for every real `m > n > 0`, so with `ν = n/m ∈
//! (0, 1)` the normalised sides are `a(ν) = 1 − ν²`, `b(ν) = 2ν`, `c(ν) = 1 + ν²`, and the system is the normalised
//! Burrau convention's at those sides ([`crate::burrau::System::of_sides`], the same formulae the discrete triples
//! take): masses, positions and Jacobi vectors are rational in `ν`, smooth on `(0, 1)`, and the primitive triples sit at
//! `ν = n/m`. The acute angle is `θ(ν) = arctan((1 − ν²)/(2ν))` ([`acute_angle`]).
//!
//! **Into the shared decoder** ([`RestBlocks`], [`BurrauBlocks`]). The decode runs the shared order mass → config → momentum →
//! derived invariants ([`crate::decode::decode_with`]): the mass block is the shared entry for a chart that sets masses
//! ([`mass::from_masses`], whose `M01_TINY` test applies), the configuration the shared canonical-frame decode of an
//! explicit `(ρ, λ)` ([`config::from_jacobi`]: mass-weighted, scale-gauged to `I = 1`, reconstructed by the decoder's
//! own reconstruction), and the derived invariants the shared descriptor. The embed's `ρ` lies on `+x` and `λ_y =
//! b/c > 0`, so the canonical frame holds by construction and `C` is a no-op. `Φ`'s [`ChartOut`] carries the zero
//! vector, which none of these blocks reads: the Burrau system is the blocks' own input. The Euclid plane overrides
//! [`Chart::decode`] with [`decode_plane`], so every entry decodes it so. The `(ν, K)` chart's construction needs the
//! exact-sign arithmetic ([`ExactSign`]) beyond [`Chart::decode`]'s bound, so, as on the invariant charts, its decode is
//! [`decode()`], which its [`super::ChartDecode`] calls, and [`Chart::decode`] on it would be the free decode of the
//! zero vector, not the chart's.
//!
//! **The `(ν, K)` chart** ([`Burrau`]; §4.3, §4.5; R-113). `ν(u) = ν_min + (ν_max − ν_min)·u`, and `K` from §2's
//! construction at `K(v) = K_max·v^γ_K`, the invariant charts' warp, its `K_max` and `γ_K` REQ-CHART-044's
//! ([`super::invariant::kinetic`]). `L_z = 0`: the chart is §4.5's bifurcation strip `Φ_strip,K = Φ_θ,K`, which takes
//! `(L_z = 0, K(v))`, with `ν` in place of `θ`. So `K_min = 0` and every `v ≥ 0` is feasible. Energy is fixed by the
//! construction, so the chart forbids `η_E` (chart_reference §0.6; R-25).
//!
//! **The seed is decided on `ν`** ([`seed`]; R-82, R-34). The embed's configuration is a canonical-frame one at
//! `I = 1`, so the construction's four seed norms are `cos²α`, `sin²α` and twice `cos²α·sin²α`
//! ([`crate::decode::invariant`]'s note): R-82 takes `(ρ, 0)` where `cos²α ≥ sin²α`, else `(0, λ)`. On the embed
//! `‖λ̃‖²/‖ρ̃‖² = b(a² + (b+c)²)/(a·c·(a+b+c)) = 2ν/(1 − ν)`, so `(ρ, 0)` is taken exactly where `3ν ≤ 1`. That is
//! decided on `ν`'s own value, exactly ([`seed`]: `3ν` as an exact sum of two floats), not on the computed norms, which
//! would fork near `ν = 1/3` where the seeds are orthogonal: one `ν` takes one seed at every precision and on every
//! backend that keeps its additions correctly rounded and unreassociated. `ν = 1/3` is `(m, n) = (3, 1)`, the
//! leg-swapped classical triangle.
//!
//! **The Euclid plane** ([`EuclidPlane`]; §4.5; chart_decoder_contract Part 5). `m(u) = m_min·(m_max/m_min)^u` and
//! `ν(v) = ν_min + (ν_max − ν_min)·v` (defaults `[1, 32]` and `[1/32, 31/32]`, the constants register's
//! `euclid_m_*` and `euclid_nu_*`), `Φ_Euclid(u, v) = (ν(v), m_Burrau(ν(v)), rest)`. The decoder uses only `ν`:
//! [`decode_plane`] takes `v` alone, so varying `u` at fixed `v` gives identical ICs by construction. `m` is an
//! annotation axis ([`EuclidPlane::axes`]): a display and landmark coordinate for the integer-lattice and
//! primitive-triple overlays ([`EuclidPlane::lattice`]), touching no block. The chart is a 1-swept family rendered in
//! 2D, well-posed as 1 + 7, and [`super::admit_statistic`] refuses a 2D density across it.
//!
//! **`system_image`** ([`nu_image`]; chart_reference §4.5; R-27, R-104, R-157). A `ν`-axis chart is
//! [`SystemImage::DoubleCover`], the quotient "shape × labelling", where its `ν` range reaches both leg orderings,
//! `ν_min < √2 − 1 < ν_max`: the leg swap `ν ↦ (1 − ν)/(1 + ν)` has the isosceles triangle `√2 − 1` as its fixed point,
//! so such a range holds some shape twice, as two labelled systems. The Euclid plane's default range does. A range on
//! one side of `√2 − 1` is bijective. Each chart's [`ChartLabel`] shows it, and `super::admit_shape_fraction` refuses a
//! shape fraction on a `DoubleCover` chart.
//!
//! **Validation** (R-26; chart_reference §5.1). Layer 2 is `ν ∈ (0, 1)` ([`Constraint::BurrauNu`]); outside it a side
//! is zero or negative, so a mass is, and clamping `ν` onto the open interval's ends would give such a system, a
//! qualitatively different IC: rejected. On `(ν, K)`, `v < 0` (`K < 0`, and no real `v^γ_K` for a non-integer `γ_K`)
//! is projected to `v = 0`, the rest start ([`Constraint::KNonNegative`]); `v > 1` is feasible (`K > K_max`) and valid,
//! the axis's extension type being TASK-M2-11's (REQ-CHART-055). Layer 3, decode sanity: the masses are positive at
//! every `ν ∈ (0, 1)` (`2ν > 0`, and `1 − ν² > 0` rounds to a positive float below 1), and the CoM and `Σpᵢ` are those
//! of a finite decode, which the reconstruction and the construction put at 0 within rounding, so the f64 decode is
//! checked for finiteness. Its positions fail only where `ν` is so small that `μ_ρ = m₀m₁/M₀₁` underflows to 0 (f64
//! subnormal `ν`) and the reconstruction divides by it: rejected, naming [`Constraint::ComAtOrigin`]. Its momenta fail
//! only where `K(v)` overflows (`v` far above 1): rejected, naming [`Constraint::MomentumZero`].

use spirv_std::num_traits::Float;

use super::invariant::{self, WarpReal};
use super::{
    AxisMeta, BlockTouch, Chart, ChartLabel, ChartOut, Constraint, SystemImage, ValidationResult,
};
use crate::burrau::{self as family, Convention, System, CONVENTION, CONVENTION_STATEMENT};
use crate::decode::config::{self, Config};
use crate::decode::invariant::{construct_seeded, ExactSign, Outcome, Seed};
use crate::decode::mass::{self, Masses};
use crate::decode::momentum::Momentum;
use crate::decode::{self, descriptor, Blocks, Decoded, Ic, Latent};
use crate::generated::links::{LinkReal, Selection};
use crate::payload::{ICDescriptorOf, PayloadReal};

/// `t` where `c` holds, else `f`: both operands are computed before the choice, so it is a select, not a branch.
#[inline]
pub(super) fn select<T: Copy>(c: bool, t: T, f: T) -> T {
    if c {
        t
    } else {
        f
    }
}

/// The embed `γ(ν)`: the normalised Burrau system at `a = 1 − ν²`, `b = 2ν`, `c = 1 + ν²` (the module's note).
#[inline]
pub fn embed<R: Float>(nu: R) -> System<R> {
    let one = R::one();
    let nn = nu * nu;
    System::of_sides(one - nn, nu + nu, one + nn)
}

/// The acute angle `θ(ν) = arctan((1 − ν²)/(2ν))`, as `atan2(1 − ν², 2ν)`, which is that for `ν > 0`: the angle at
/// `r₂` (`tan θ = a/b`).
#[inline]
pub fn acute_angle<R: Float>(nu: R) -> R {
    Float::atan2(R::one() - nu * nu, nu + nu)
}

/// The seed R-82's rule takes on the embed at `ν`: `(ρ, 0)` where `3ν ≤ 1`, else `(0, λ)` (the module's note). `3ν`
/// is formed exactly as `s + e`, `s = 2ν + ν` rounded and `e` its error by the two-sum, and `3ν ≤ 1` is `s < 1`, or
/// `s = 1` and `e ≤ 0`: only additions, subtractions and comparisons, so one `ν` takes one seed on every backend.
#[inline]
pub fn seed<R: Float>(nu: R) -> Seed {
    let two = nu + nu;
    let s = two + nu;
    let back = s - two;
    let e = (two - (s - back)) + (nu - back);
    let one = R::one();
    let at_most = (s < one) | ((s == one) & (e <= R::zero()));
    select(at_most, Seed::Rho, Seed::Lambda)
}

/// `lo + (hi − lo)·t`.
#[inline]
pub(super) fn lerp<F: Float>(lo: F, hi: F, t: F) -> F {
    lo + (hi - lo) * t
}

/// The system's masses, through the shared entry for a chart that sets masses ([`mass::from_masses`]).
#[inline]
pub(super) fn system_masses<R: LinkReal>(system: &System<R>) -> Masses<R> {
    mass::from_masses(system.m)
}

/// The system's configuration, through the shared canonical-frame entry for an explicit `(ρ, λ)`
/// ([`config::from_jacobi`]).
#[inline]
pub(super) fn system_config<R: LinkReal>(system: &System<R>, masses: &Masses<R>) -> Config<R> {
    config::from_jacobi(masses, system.rho, system.lambda)
}

/// The Euclid plane's decoder, the Burrau system at rest: its masses and configuration through the shared entries,
/// every momentum 0, and the shared descriptor. No block reads its `z` slice.
pub struct RestBlocks<R> {
    /// The system the embed gave.
    system: System<R>,
}

impl<R> RestBlocks<R> {
    /// The decoder of `system` at rest.
    #[inline]
    pub fn new(system: System<R>) -> Self {
        RestBlocks { system }
    }
}

impl<R: LinkReal + PayloadReal> Blocks<R> for RestBlocks<R> {
    #[inline]
    fn mass(&mut self, _z_mass: [R; 2]) -> Masses<R> {
        system_masses(&self.system)
    }

    #[inline]
    fn config(&mut self, _z_cfg: [R; 2], masses: &Masses<R>) -> Config<R> {
        system_config(&self.system, masses)
    }

    /// The rest start: every momentum 0.
    #[inline]
    fn momentum(
        &mut self,
        _z_mom: [R; 4],
        _masses: &Masses<R>,
        _config: &Config<R>,
    ) -> Momentum<R> {
        let zero = R::zero();
        Momentum {
            q: [zero; 4],
            p_rho: [zero; 2],
            p_lambda: [zero; 2],
            p: [[zero; 2]; 3],
        }
    }

    #[inline]
    fn invariants(&mut self, ic: &Ic<R>) -> ICDescriptorOf<R> {
        descriptor::describe(ic)
    }
}

/// The `(ν, K)` chart's decoder: the system's masses and configuration as [`RestBlocks`] takes them, the momenta by
/// §2's construction at the targets `(L_z, K*)` with the seed given, and the shared descriptor. No block reads its `z`
/// slice.
pub struct BurrauBlocks<R> {
    /// The system the embed gave.
    system: System<R>,
    /// The construction's seed, [`seed`]'s.
    seed: Seed,
    /// The target `L_z`.
    lz: R,
    /// The target `K*`.
    k: R,
    /// What the construction did.
    outcome: Outcome,
}

impl<R> BurrauBlocks<R> {
    /// The decoder of `system` with the construction at `L_z = lz`, `K* = k`, its seed `seed`.
    #[inline]
    pub fn new(system: System<R>, seed: Seed, lz: R, k: R) -> Self {
        BurrauBlocks {
            system,
            seed,
            lz,
            k,
            outcome: Outcome::Feasible,
        }
    }

    /// What the construction did, once the momentum block has run.
    #[inline]
    pub fn outcome(&self) -> Outcome {
        self.outcome
    }
}

impl<R: LinkReal + PayloadReal + ExactSign> Blocks<R> for BurrauBlocks<R> {
    #[inline]
    fn mass(&mut self, _z_mass: [R; 2]) -> Masses<R> {
        system_masses(&self.system)
    }

    #[inline]
    fn config(&mut self, _z_cfg: [R; 2], masses: &Masses<R>) -> Config<R> {
        system_config(&self.system, masses)
    }

    /// The construction (chart_reference §2.2), whose [`Momentum::q`] holds the Jacobi momenta `(p_ρ, p_λ)`, as on the
    /// invariant charts.
    #[inline]
    fn momentum(&mut self, _z_mom: [R; 4], masses: &Masses<R>, config: &Config<R>) -> Momentum<R> {
        let c = construct_seeded(masses, config, self.seed, self.lz, self.k);
        self.outcome = c.outcome;
        invariant::jacobi(masses, c.p)
    }

    #[inline]
    fn invariants(&mut self, ic: &Ic<R>) -> ICDescriptorOf<R> {
        descriptor::describe(ic)
    }
}

/// The zero latent vector, the Burrau charts' [`ChartOut`] (the module's note).
#[inline]
pub(super) fn unread<F: Float>() -> ChartOut<F> {
    ChartOut {
        z: Latent::new([F::zero(); 8]),
    }
}

/// The layer-3 check on the f64 decode `d` (the module's note): `None` where it is sane. The masses are positive at
/// every `ν ∈ (0, 1)`, so only the positions and the momenta are read.
pub(super) fn sanity(d: &Decoded<f64>) -> Option<Constraint> {
    let finite = |v: &[[f64; 2]; 3]| v.iter().flatten().all(|x| x.is_finite());
    if !finite(&d.ic.r) {
        Some(Constraint::ComAtOrigin)
    } else if !finite(&d.ic.p) {
        Some(Constraint::MomentumZero)
    } else {
        None
    }
}

/// Layer 2's `ν ∈ (0, 1)`.
#[inline]
fn in_family(nu: f64) -> bool {
    nu > 0.0 && nu < 1.0
}

/// The leg swap's fixed point `ν = √2 − 1`, the isosceles triangle (`a = b`): `θ(ν) = π/4`.
#[inline]
pub fn isosceles_nu() -> f64 {
    Float::sqrt(2.0) - 1.0
}

/// The `system_image` of a `ν`-axis chart on `[ν_min, ν_max]` (the module's note): [`SystemImage::DoubleCover`] where
/// `ν_min < √2 − 1 < ν_max`, the range reaching both leg orderings, else [`SystemImage::Bijective`].
#[inline]
pub fn nu_image(nu_range: [f64; 2]) -> SystemImage {
    let fixed = isosceles_nu();
    select(
        nu_range[0] < fixed && fixed < nu_range[1],
        SystemImage::DoubleCover,
        SystemImage::Bijective,
    )
}

/// The quotient a Burrau chart of `image` covers, in words, for its [`ChartLabel`] (chart_reference §4.5): "shape ×
/// labelling" for `DoubleCover`, else "shape only". No digit, so no body number (R-449).
#[inline]
pub fn quotient(image: SystemImage) -> &'static str {
    match image {
        SystemImage::DoubleCover => {
            "shape × labelling: both leg orderings, a shape covered twice as two labelled systems"
        }
        _ => "shape only: one leg ordering, each shape covered once",
    }
}

/// The [`ChartLabel`] of a Burrau chart named `name` with `system_image` `image`.
#[inline]
pub fn label(name: &str, image: SystemImage) -> ChartLabel<'_> {
    ChartLabel {
        name,
        image,
        quotient: quotient(image),
    }
}

// ── The (ν, K) chart ───────────────────────────────────────────────────────────────────────────────────────────────

/// The `(ν, K)` chart (chart_reference §4.3, §4.5, §5.1's `Burrau { nu_range, k_range }`; R-113): `ν(u)` on
/// `nu_range`, `K(v) = K_max·v^γ_K`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Burrau<P> {
    /// `[ν_min, ν_max]`, inside `(0, 1)`.
    pub nu_range: [P; 2],
    /// `K_max > 0`, the kinetic energy at `v = 1`.
    pub k_max: P,
    /// `γ_K ≥ 1`, the warp's exponent.
    pub gamma_k: P,
}

impl<P: Copy> Burrau<P> {
    /// `ν(u) = ν_min + (ν_max − ν_min)·u`.
    #[inline]
    pub fn nu<F: Float + From<P>>(&self, u: F) -> F {
        let wide = <F as From<P>>::from;
        lerp(wide(self.nu_range[0]), wide(self.nu_range[1]), u)
    }

    /// `K(v) = K_max·v^γ_K`.
    #[inline]
    pub fn kinetic<F: Float + From<P>>(&self, v: F) -> F {
        let wide = <F as From<P>>::from;
        invariant::kinetic(wide(self.k_max), wide(self.gamma_k), v)
    }

    /// The convention the chart's systems are stated in: the normalised one (chart_reference §4.2).
    #[inline]
    pub fn convention(&self) -> Convention {
        CONVENTION
    }

    /// [`Burrau::convention`] in words.
    #[inline]
    pub fn convention_statement(&self) -> &'static str {
        CONVENTION_STATEMENT
    }

    /// The axes' metadata (chart_decoder_contract Part 5): `ν` writes the configuration and the masses, `K` the
    /// momenta; neither is annotation-only, so the chart is 2-swept.
    #[inline]
    pub fn axes(&self) -> [AxisMeta; 2] {
        [
            AxisMeta {
                touches: BlockTouch {
                    config: true,
                    momentum: false,
                    mass: true,
                },
                annotation: false,
            },
            AxisMeta {
                touches: BlockTouch {
                    config: false,
                    momentum: true,
                    mass: false,
                },
                annotation: false,
            },
        ]
    }
}

impl<P: Copy> Burrau<P>
where
    f64: From<P>,
{
    /// The chart's `system_image`, [`nu_image`] of its `ν` range (the module's note).
    #[inline]
    pub fn system_image(&self) -> SystemImage {
        nu_image(self.nu_range.map(<f64 as From<P>>::from))
    }

    /// The chart's label: its name, its `system_image` and the quotient it covers.
    #[inline]
    pub fn label(&self) -> ChartLabel<'_> {
        label(self.name(), self.system_image())
    }
}

impl<P: Copy> Chart for Burrau<P>
where
    f64: From<P>,
{
    type Param = P;

    /// The zero vector, unread (the module's note).
    #[inline]
    fn map<F: Float + From<P>>(&self, _u: F, _v: F) -> ChartOut<F> {
        unread()
    }

    /// `true`: the construction fixes the energy (chart_reference §0.6).
    fn forbids_energy_normalisation(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "Burrau"
    }

    /// Layer 2's `ν ∈ (0, 1)` and `K ≥ 0`, then layer 3 on the f64 decode (the module's note).
    fn validate(&self, u: f64, v: f64) -> ValidationResult {
        if !(u.is_finite() && v.is_finite()) {
            return ValidationResult::Reject {
                failed: select(
                    u.is_finite(),
                    Constraint::KNonNegative,
                    Constraint::BurrauNu,
                ),
            };
        }
        if !in_family(self.nu(u)) {
            return ValidationResult::Reject {
                failed: Constraint::BurrauNu,
            };
        }
        let t = v.max(0.0);
        match sanity(&decode(self, u, t).0) {
            Some(failed) => ValidationResult::Reject { failed },
            None if t != v => ValidationResult::Project {
                u,
                v: t,
                failed: Constraint::KNonNegative,
            },
            None => ValidationResult::Valid,
        }
    }
}

/// `D ∘ Φ` at `(u, v)` on the `(ν, K)` chart: the embed at `ν(u)`, §2's construction at `L_z = 0`, `K* = K(v)`, its
/// seed [`seed`]`(ν)`; and the construction's outcome.
#[inline]
pub fn decode<R, P>(chart: &Burrau<P>, u: R, v: R) -> (Decoded<R>, Outcome)
where
    R: LinkReal + PayloadReal + WarpReal + From<P>,
    P: Copy,
{
    let nu = chart.nu(u);
    let mut blocks = BurrauBlocks::new(embed(nu), seed(nu), R::zero(), chart.kinetic(v));
    let decoded = decode::decode_with(&mut blocks, &unread().z);
    (decoded, blocks.outcome())
}

// ── The Euclid plane ───────────────────────────────────────────────────────────────────────────────────────────────

/// The Euclid plane (chart_reference §4.5): `m(u) = m_min·(m_max/m_min)^u`, an annotation axis, and `ν(v)` on
/// `nu_range`, the only coordinate the decoder reads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EuclidPlane<P> {
    /// `[m_min, m_max]`, `0 < m_min < m_max`.
    pub m_range: [P; 2],
    /// `[ν_min, ν_max]`, inside `(0, 1)`.
    pub nu_range: [P; 2],
}

impl<P: Copy> EuclidPlane<P> {
    /// `m(u) = m_min·(m_max/m_min)^u`, the annotation coordinate.
    #[inline]
    pub fn m<F: Float + From<P>>(&self, u: F) -> F {
        let wide = <F as From<P>>::from;
        let (lo, hi) = (wide(self.m_range[0]), wide(self.m_range[1]));
        lo * Float::powf(hi / lo, u)
    }

    /// `ν(v) = ν_min + (ν_max − ν_min)·v`.
    #[inline]
    pub fn nu<F: Float + From<P>>(&self, v: F) -> F {
        let wide = <F as From<P>>::from;
        lerp(wide(self.nu_range[0]), wide(self.nu_range[1]), v)
    }

    /// The convention the chart's systems are stated in: the normalised one (chart_reference §4.2).
    #[inline]
    pub fn convention(&self) -> Convention {
        CONVENTION
    }

    /// [`EuclidPlane::convention`] in words.
    #[inline]
    pub fn convention_statement(&self) -> &'static str {
        CONVENTION_STATEMENT
    }

    /// The axes' metadata (chart_decoder_contract Part 5): `m` annotation-only, touching no block; `ν` writes the
    /// configuration and the masses. The momenta are the rest convention's, so the chart is 1 swept + 7 frozen.
    #[inline]
    pub fn axes(&self) -> [AxisMeta; 2] {
        [
            AxisMeta {
                touches: BlockTouch {
                    config: false,
                    momentum: false,
                    mass: false,
                },
                annotation: true,
            },
            AxisMeta {
                touches: BlockTouch {
                    config: true,
                    momentum: false,
                    mass: true,
                },
                annotation: false,
            },
        ]
    }
}

/// A lattice point of the Euclid plane's overlays (chart_reference §4.5): integers `m > n > 0` and where they sit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LatticePoint {
    /// `m`.
    pub m: u32,
    /// `n`.
    pub n: u32,
    /// The horizontal coordinate where `m(u) = m`.
    pub u: f64,
    /// The vertical coordinate where `ν(v) = n/m`.
    pub v: f64,
    /// Whether `(m, n)` gives a primitive triple ([`family::primitive`]): the primitive-triple overlay's points.
    pub primitive: bool,
}

impl<P: Copy> EuclidPlane<P>
where
    f64: From<P>,
{
    /// The integer lattice inside the chart: every `(m, n)`, `m, n ∈ ℤ`, `m > n > 0`, with `m_min ≤ m ≤ m_max` and
    /// `ν_min ≤ n/m ≤ ν_max`, at its `(u, v)`; the primitive-triple overlay is its points with
    /// [`LatticePoint::primitive`] set.
    pub fn lattice(&self) -> impl Iterator<Item = LatticePoint> {
        let wide = <f64 as From<P>>::from;
        let (m_lo, m_hi) = (wide(self.m_range[0]), wide(self.m_range[1]));
        let (nu_lo, nu_hi) = (wide(self.nu_range[0]), wide(self.nu_range[1]));
        let first = Float::max(Float::ceil(m_lo), 1.0) as u32;
        let last = Float::floor(m_hi) as u32;
        (first..=last).flat_map(move |m| {
            (1..m).filter_map(move |n| {
                let nu = n as f64 / m as f64;
                (nu >= nu_lo && nu <= nu_hi).then(|| LatticePoint {
                    m,
                    n,
                    u: Float::ln(m as f64 / m_lo) / Float::ln(m_hi / m_lo),
                    v: (nu - nu_lo) / (nu_hi - nu_lo),
                    primitive: family::primitive(m, n),
                })
            })
        })
    }
}

impl<P: Copy> EuclidPlane<P>
where
    f64: From<P>,
{
    /// The chart's `system_image`, [`nu_image`] of its `ν` range (the module's note): `DoubleCover` at the default
    /// range, the full-range chart of R-27 and R-157.
    #[inline]
    pub fn system_image(&self) -> SystemImage {
        nu_image(self.nu_range.map(<f64 as From<P>>::from))
    }

    /// The chart's label: its name, its `system_image` and the quotient it covers.
    #[inline]
    pub fn label(&self) -> ChartLabel<'_> {
        label(self.name(), self.system_image())
    }
}

impl<P: Copy> Chart for EuclidPlane<P>
where
    f64: From<P>,
{
    type Param = P;

    /// The zero vector, unread (the module's note).
    #[inline]
    fn map<F: Float + From<P>>(&self, _u: F, _v: F) -> ChartOut<F> {
        unread()
    }

    fn name(&self) -> &str {
        "EuclidPlane"
    }

    /// [`decode_plane`] at `v`: `u` is not read.
    #[inline]
    fn decode<S, R>(&self, _u: R, v: R) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        decode_plane(self, v)
    }

    /// Layer 2's `ν ∈ (0, 1)`, then layer 3 on the f64 decode (the module's note); `u` is not read, as the decoder
    /// does not read it.
    fn validate(&self, _u: f64, v: f64) -> ValidationResult {
        if !(v.is_finite() && in_family(self.nu(v))) {
            return ValidationResult::Reject {
                failed: Constraint::BurrauNu,
            };
        }
        match sanity(&decode_plane(self, v)) {
            Some(failed) => ValidationResult::Reject { failed },
            None => ValidationResult::Valid,
        }
    }
}

/// `D ∘ Φ_Euclid` at `v`: the embed at `ν(v)`, at rest. The annotation coordinate `u` is not an argument: the decoder
/// uses only `ν`.
#[inline]
pub fn decode_plane<R, P>(chart: &EuclidPlane<P>, v: R) -> Decoded<R>
where
    R: LinkReal + PayloadReal + From<P>,
    P: Copy,
{
    let mut blocks = RestBlocks::new(embed(chart.nu(v)));
    decode::decode_with(&mut blocks, &unread().z)
}
