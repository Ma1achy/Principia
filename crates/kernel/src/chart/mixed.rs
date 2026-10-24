//! `Mixed<H, V>`, the mixed-axis charts (chart_reference §1.3; chart_decoder_contract § "The four axis kinds";
//! REQ-CHART-008): the two axes need not come from the same block, and any pair of quantities can be the horizontal and
//! vertical axes, the other six frozen at stated values.
//!
//! **Each axis states what it maps and its warp** ([`Quantity`]). An axis is a latent coordinate `z_k`, kind 1
//! ([`ZAlpha`] … [`ZMu2`]), or the derived quantity one control's link produces, the decoder's own value: `α`, `β` and
//! the Jacobi momenta's components `p_ρ = (q₀, q₁)`, `p_λ = (q₂, q₃)` ([`Alpha`], [`Beta`], [`PRhoX`], [`PRhoY`],
//! [`PLambdaX`], [`PLambdaY`]), each in its own block, set by the link's inverse, so kind 2 with no residual: the block's
//! other control is frozen. The mass block's one link maps two controls onto three masses, so a single mass has no
//! inverse of its own; a mass axis is a latent mass control, `z_μ₁` or `z_μ₂`. The warp is affine from `[0,1]` onto the
//! axis's range, `x(t) = lo + (hi − lo)·t`, in `z` for a latent coordinate and in the quantity for a derived one (R-408's
//! "a physical-quantity axis mapped linearly onto its range"). Which quantity each axis is, is the chart's type, baked
//! (lowering Part 3: chart type and axis kinds are monomorphised, never interpreted); the ranges and the frozen values
//! are its parameters, uniforms.
//!
//! **The convention fixes the other six** ([`Mixed::frozen`]): the eight controls `z`, in the decoder's block layout,
//! of which the two the axes set are not read. Then `Φ_mixed(u, v) = D(z(u, v))`, `z(u, v)` taking the horizontal
//! axis's coordinate from `u`, the vertical's from `v` and the rest from the frozen values ([`Mixed::controls`]). It
//! composes with the shared decoder ([`crate::decode::decode`]) and adds no GPU logic: the decode is the latent chart's
//! on that `z`. A derived axis's control is its link's inverse at the axis's value, which needs the link selection, so
//! [`Chart::decode`] places it; [`Chart::map`] places the latent axes and leaves a derived axis's coordinate frozen,
//! and every `D ∘ Φ` entry goes through [`Chart::decode`]. The `(α, p_ρ)` chart, chart_reference §1.3's classical
//! phase-space portrait, is `Mixed<Alpha, PRhoX>`: `p_ρ`'s component along `ρ̃`, which the canonical frame puts on
//! `+x`. That component, `q₀`, is the momentum conjugate to the inner pair's separation `‖ρ‖ = cos α/√μ_ρ`
//! (chart_reference §0.2, §0.3), not to `α`, whose conjugate is `p_α = −sin α·(p̃_ρ·ρ̂) + cos α·(p̃_λ·λ̂)`; the axes
//! are not a canonical pair, and `dα·dp_ρ,x` is not the Liouville measure (canonical_spec §9 W7; chart_decoder_contract
//! § "Links carry a measure").
//!
//! **Feasibility is inherited** (inverse_encode_contract § "Chart-aware validation", layer 2: "Mixed-axis charts inherit
//! both axes' constraints"). Each axis has its own check, and the chart's is their conjunction: a latent coordinate
//! takes any finite `z`, which the decoder accepts (R-407), and a derived quantity must lie in its domain, its link's
//! codomain, closed (`α ∈ [0, π/2]`, `β ∈ [0, π]`, each momentum component in `[−q_max, q_max]`;
//! [`Constraint::AxisDomain`]). A value past its domain is clamped onto it, the coordinate snapped to where the axis
//! meets the edge, as layer 1 clamps an input past the decoder's range (`lookup_clamped`), and `moved` says which axes
//! moved; a non-finite coordinate, or an axis whose range has no width to clamp along, is rejected. Layer 3 holds by the
//! shared decoder's construction, as on the latent chart: every control the check admits is finite or a link's
//! saturated inverse at its domain's edge, which the link maps back onto the edge.
//!
//! **The active values** (chart_reference §0.1, "every chart reads the active value"; chart_decoder_contract § "Three
//! hard requirements on any registered link", "Tunable parameters"; R-481 G4; REQ-DEC-048). A momentum axis's domain
//! is `[−q_max, q_max]` at the active `q_max` ([`Quantity::domain_at`], the codomain's [`Ends::ends_at`]), its control
//! the link's inverse at it ([`Quantity::control_at`]), and the decode reads the active `μ_max` and `q_max`
//! ([`Chart::decode_params`], [`Chart::validate_params`]); so an axis labelled `x` decodes to `x` at every value of a
//! declared range. The forms without the parameters are these at `LinkParams::DEFAULT`.
//!
//! **Per-pixel mass** (inverse_encode_contract's chart flags): a chart with a mass axis has masses that vary per pixel
//! and declares `requires_per_pixel_mass`; one with none holds its masses at the frozen mass controls.

use core::fmt;
use core::marker::PhantomData;

use spirv_std::num_traits::Float;

use super::invariant::WarpReal;
use super::{AxisMeta, BlockTouch, Chart, ChartDecode, ChartOut, Constraint, ValidationResult};
use crate::decode::{self, Decoded, Latent};
use crate::encode::blocks::Ends;
use crate::generated::links::{
    Alpha as AlphaRange, Beta as BetaRange, Codomain, Link, LinkParams, LinkReal,
    Momentum as MomentumRange, Selection,
};
use crate::payload::PayloadReal;

/// One of the eight latent coordinates, in the decoder's block layout (`crate::decode::latent`). Its discriminant is a
/// `u32`, a type the kernel's SPIR-V has without the `Int8` capability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Coordinate {
    /// `z[0]`, `z_α`, configuration.
    ZAlpha,
    /// `z[1]`, `z_β`, configuration.
    ZBeta,
    /// `z[2]`, `z_q0`, momentum.
    ZQ0,
    /// `z[3]`, `z_q1`, momentum.
    ZQ1,
    /// `z[4]`, `z_q2`, momentum.
    ZQ2,
    /// `z[5]`, `z_q3`, momentum.
    ZQ3,
    /// `z[6]`, `z_μ1`, mass.
    ZMu1,
    /// `z[7]`, `z_μ2`, mass.
    ZMu2,
}

impl Coordinate {
    /// The decoder block the coordinate is in (chart_decoder_contract Part 5's block-touch set).
    pub fn touches(self) -> BlockTouch {
        use Coordinate::*;
        BlockTouch {
            config: matches!(self, ZAlpha | ZBeta),
            momentum: matches!(self, ZQ0 | ZQ1 | ZQ2 | ZQ3),
            mass: matches!(self, ZMu1 | ZMu2),
        }
    }
}

/// What a mixed chart's axis maps (the module's note): the latent coordinate it sets and how.
pub trait Quantity {
    /// The coordinate the axis sets.
    const COORDINATE: Coordinate;
    /// The axis's domain, closed, at the default parameters: its link's codomain for a derived quantity, the whole
    /// line for a latent coordinate ([`Quantity::domain_at`] at `LinkParams::DEFAULT`).
    const DOMAIN: [f64; 2];
    /// Whether the axis is a derived quantity, set through its link's inverse, or a latent coordinate, set directly.
    const DERIVED: bool;
    /// The axis's name, for the chart's label.
    const NAME: &'static str;
    /// The control `z_k` that gives the axis's value `x` under the selection `S`: `x` itself for a latent coordinate,
    /// the link's inverse for a derived quantity ([`Quantity::control_at`] at `LinkParams::DEFAULT`).
    fn control<S: Selection, R: LinkReal>(x: R) -> R;

    /// [`Quantity::control`] at the tunable parameters `p`: a derived quantity's link inverse at the active values; a
    /// latent coordinate reads none.
    #[inline]
    fn control_at<S: Selection, R: LinkReal>(x: R, _p: &LinkParams<R>) -> R {
        Self::control::<S, R>(x)
    }

    /// The axis's domain at the tunable parameters `p` (host-side, f64): a momentum axis's `[−q_max, q_max]` at the
    /// active `q_max`; an axis whose domain reads none is [`Quantity::DOMAIN`].
    #[inline]
    fn domain_at(_p: &LinkParams<f64>) -> [f64; 2] {
        Self::DOMAIN
    }
}

/// The latent coordinates' axes, kind 1: `z_k` itself, any finite value.
macro_rules! latent_axes {
    ($($(#[$doc:meta])* $axis:ident => $coordinate:ident, $name:literal;)*) => {$(
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct $axis;

        impl Quantity for $axis {
            const COORDINATE: Coordinate = Coordinate::$coordinate;
            const DOMAIN: [f64; 2] = [f64::NEG_INFINITY, f64::INFINITY];
            const DERIVED: bool = false;
            const NAME: &'static str = $name;
            #[inline]
            fn control<S: Selection, R: LinkReal>(x: R) -> R {
                x
            }
        }
    )*};
}

latent_axes! {
    /// The axis `z_α`.
    ZAlpha => ZAlpha, "z_alpha";
    /// The axis `z_β`.
    ZBeta => ZBeta, "z_beta";
    /// The axis `z_q0`.
    ZQ0 => ZQ0, "z_q0";
    /// The axis `z_q1`.
    ZQ1 => ZQ1, "z_q1";
    /// The axis `z_q2`.
    ZQ2 => ZQ2, "z_q2";
    /// The axis `z_q3`.
    ZQ3 => ZQ3, "z_q3";
    /// The axis `z_μ1`, a mass axis.
    ZMu1 => ZMu1, "z_mu1";
    /// The axis `z_μ2`, a mass axis.
    ZMu2 => ZMu2, "z_mu2";
}

/// The derived quantities' axes, kind 2: one control's link output, set by the link's inverse.
macro_rules! derived_axes {
    ($($(#[$doc:meta])* $axis:ident => $coordinate:ident, $range:ident, $link:ident, $name:literal;)*) => {$(
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct $axis;

        impl Quantity for $axis {
            const COORDINATE: Coordinate = Coordinate::$coordinate;
            const DOMAIN: [f64; 2] = [<$range as Codomain>::LO, <$range as Codomain>::HI];
            const DERIVED: bool = true;
            const NAME: &'static str = $name;
            #[inline]
            fn control<S: Selection, R: LinkReal>(x: R) -> R {
                Self::control_at::<S, R>(x, &LinkParams::DEFAULT)
            }

            #[inline]
            fn control_at<S: Selection, R: LinkReal>(x: R, p: &LinkParams<R>) -> R {
                let [z] = <S::$link as Link<1, 1>>::inverse_at([x], p);
                z
            }

            /// The codomain's ends at the active values ([`Ends::ends_at`]): at the defaults, [`Quantity::DOMAIN`].
            #[inline]
            fn domain_at(p: &LinkParams<f64>) -> [f64; 2] {
                let (lo, hi) = <$range as Ends>::ends_at::<f64>(p);
                [lo, hi]
            }
        }
    )*};
}

derived_axes! {
    /// The axis `α`, the configuration's hyperangle, `‖ρ̃‖ = cos α`.
    Alpha => ZAlpha, AlphaRange, AlphaLink, "alpha";
    /// The axis `β`, `λ̃`'s angle from `+x`.
    Beta => ZBeta, BetaRange, BetaLink, "beta";
    /// The axis `p_ρ,x = q₀`, the inner pair's Jacobi momentum along `ρ̃`.
    PRhoX => ZQ0, MomentumRange, Q0Link, "p_rho_x";
    /// The axis `p_ρ,y = q₁`.
    PRhoY => ZQ1, MomentumRange, Q1Link, "p_rho_y";
    /// The axis `p_λ,x = q₂`.
    PLambdaX => ZQ2, MomentumRange, Q2Link, "p_lambda_x";
    /// The axis `p_λ,y = q₃`.
    PLambdaY => ZQ3, MomentumRange, Q3Link, "p_lambda_y";
}

/// A mixed-axis chart (the module's note): the horizontal axis `H` on `h_range`, the vertical `V` on `v_range`, the
/// other six controls frozen.
pub struct Mixed<H, V, P> {
    /// The frozen controls `z`, in the decoder's block layout; the two the axes set are not read.
    pub frozen: [P; 8],
    /// `[lo, hi]`, the horizontal axis's value at `u = 0` and `u = 1`.
    pub h_range: [P; 2],
    /// `[lo, hi]`, the vertical axis's value at `v = 0` and `v = 1`.
    pub v_range: [P; 2],
    /// The axes, the chart's type.
    axes: PhantomData<(H, V)>,
}

impl<H, V, P: Copy> Clone for Mixed<H, V, P> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<H, V, P: Copy> Copy for Mixed<H, V, P> {}

impl<H, V, P: PartialEq> PartialEq for Mixed<H, V, P> {
    fn eq(&self, other: &Self) -> bool {
        self.frozen == other.frozen
            && self.h_range == other.h_range
            && self.v_range == other.v_range
    }
}

impl<H: Quantity, V: Quantity, P: fmt::Debug> fmt::Debug for Mixed<H, V, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Mixed")
            .field("h", &H::NAME)
            .field("v", &V::NAME)
            .field("frozen", &self.frozen)
            .field("h_range", &self.h_range)
            .field("v_range", &self.v_range)
            .finish()
    }
}

/// `w`, or `x` where `k` is the coordinate `at`.
#[inline]
fn pick<F>(k: Coordinate, at: Coordinate, x: F, w: F) -> F {
    if k == at {
        x
    } else {
        w
    }
}

/// `z` with the coordinate `at` set to `x`, each component read by a literal index (gpu_determinism_note rule 5).
#[inline]
fn place<F: Copy>(z: [F; 8], at: Coordinate, x: F) -> [F; 8] {
    use Coordinate::*;
    [
        pick(ZAlpha, at, x, z[0]),
        pick(ZBeta, at, x, z[1]),
        pick(ZQ0, at, x, z[2]),
        pick(ZQ1, at, x, z[3]),
        pick(ZQ2, at, x, z[4]),
        pick(ZQ3, at, x, z[5]),
        pick(ZMu1, at, x, z[6]),
        pick(ZMu2, at, x, z[7]),
    ]
}

/// `lo + (hi − lo)·t`.
#[inline]
fn lerp<F: Float>(range: [F; 2], t: F) -> F {
    range[0] + (range[1] - range[0]) * t
}

impl<H: Quantity, V: Quantity, P: Copy> Mixed<H, V, P> {
    /// The two axes set different coordinates, checked where the chart is built for its axes.
    const DISTINCT: () = assert!(
        H::COORDINATE as u32 != V::COORDINATE as u32,
        "a mixed chart's two axes set one coordinate"
    );

    /// The chart with the frozen controls `frozen` and the axes' ranges.
    #[inline]
    pub fn new(frozen: [P; 8], h_range: [P; 2], v_range: [P; 2]) -> Self {
        let () = Self::DISTINCT;
        Mixed {
            frozen,
            h_range,
            v_range,
            axes: PhantomData,
        }
    }

    /// The horizontal axis's value at `u`.
    #[inline]
    pub fn h<F: Float + From<P>>(&self, u: F) -> F {
        let w = <F as From<P>>::from;
        lerp([w(self.h_range[0]), w(self.h_range[1])], u)
    }

    /// The vertical axis's value at `v`.
    #[inline]
    pub fn v<F: Float + From<P>>(&self, v: F) -> F {
        let w = <F as From<P>>::from;
        lerp([w(self.v_range[0]), w(self.v_range[1])], v)
    }

    /// The frozen controls at `F`.
    #[inline]
    fn frozen_at<F: Float + From<P>>(&self) -> [F; 8] {
        let f = self.frozen;
        let w = <F as From<P>>::from;
        [
            w(f[0]),
            w(f[1]),
            w(f[2]),
            w(f[3]),
            w(f[4]),
            w(f[5]),
            w(f[6]),
            w(f[7]),
        ]
    }

    /// `z(u, v)` under the selection `S`: the frozen controls with each axis's coordinate set to the control that
    /// gives its value (the module's note), at the default parameters.
    #[inline]
    pub fn controls<S: Selection, R: LinkReal + From<P>>(&self, u: R, v: R) -> [R; 8] {
        self.controls_at::<S, R>(u, v, &LinkParams::DEFAULT)
    }

    /// [`Mixed::controls`] at the tunable parameters `p`: each derived axis's control the link's inverse at the active
    /// values ([`Quantity::control_at`]).
    #[inline]
    pub fn controls_at<S: Selection, R: LinkReal + From<P>>(
        &self,
        u: R,
        v: R,
        p: &LinkParams<R>,
    ) -> [R; 8] {
        let z = place(
            self.frozen_at(),
            H::COORDINATE,
            H::control_at::<S, R>(self.h(u), p),
        );
        place(z, V::COORDINATE, V::control_at::<S, R>(self.v(v), p))
    }

    /// The axes' metadata (chart_decoder_contract Part 5): each writes its coordinate's block; neither is annotation-only.
    pub fn axes(&self) -> [AxisMeta; 2] {
        [
            AxisMeta {
                touches: H::COORDINATE.touches(),
                annotation: false,
            },
            AxisMeta {
                touches: V::COORDINATE.touches(),
                annotation: false,
            },
        ]
    }
}

/// One axis's check at `t` on `range` against the domain `[lo, hi]` (the module's note): `Ok(t)` inside the domain,
/// `Ok(t′)` with `t′ ≠ t` where the axis meets the domain's edge, `Err(())` where nothing can be clamped: a value that
/// is not finite, a range of no width or of a width past f64's range, or an edge four inward steps of `t` do not reach.
fn axis_check([lo, hi]: [f64; 2], range: [f64; 2], t: f64) -> Result<f64, ()> {
    let x = lerp(range, t);
    if !x.is_finite() {
        return Err(());
    }
    if in_domain(x, lo, hi) {
        return Ok(t);
    }
    let edge = x.clamp(lo, hi);
    let below = edge == lo;
    let width = range[1] - range[0];
    // Rounding may leave `x(t)` a hair past the edge, by an ulp or two: step `t` inward, toward larger `x` from below
    // and smaller from above, until it lands in the domain, at most four steps, and refuse past them. A range of no
    // width or of a width past f64's range gives a `t` whose `x` is NaN or infinite, and is refused so.
    let up = below == width.is_sign_positive();
    let mut t = (edge - range[0]) / width;
    for _ in 0..4 {
        if in_domain(lerp(range, t), lo, hi) {
            return Ok(t);
        }
        t = if up { t.next_up() } else { t.next_down() };
    }
    Err(())
}

/// Whether `x ∈ [lo, hi]`.
#[inline]
fn in_domain(x: f64, lo: f64, hi: f64) -> bool {
    x >= lo && x <= hi
}

impl<H: Quantity, V: Quantity, P: Copy> Chart for Mixed<H, V, P>
where
    f64: From<P>,
{
    type Param = P;

    /// The frozen controls with each latent axis's coordinate placed; a derived axis's coordinate is the frozen value
    /// here, and [`Chart::decode`] sets it through the link (the module's note).
    #[inline]
    fn map<F: Float + From<P>>(&self, u: F, v: F) -> ChartOut<F> {
        let z = self.frozen_at();
        let z = if H::DERIVED {
            z
        } else {
            place(z, H::COORDINATE, self.h(u))
        };
        let z = if V::DERIVED {
            z
        } else {
            place(z, V::COORDINATE, self.v(v))
        };
        ChartOut { z: Latent::new(z) }
    }

    fn name(&self) -> &str {
        "Mixed"
    }

    /// `true` where either axis is a mass axis (the module's note).
    fn requires_per_pixel_mass(&self) -> bool {
        H::COORDINATE.touches().mass | V::COORDINATE.touches().mass
    }

    /// [`Chart::decode_params`] at the defaults.
    #[inline]
    fn decode<S, R>(&self, u: R, v: R) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        self.decode_params::<S, R>(u, v, &LinkParams::DEFAULT)
    }

    /// `D(z(u, v))`, the shared decoder on [`Mixed::controls_at`] under `S`, both at the tunable parameters `p` (the
    /// module's note).
    #[inline]
    fn decode_params<S, R>(&self, u: R, v: R, p: &LinkParams<R>) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        decode::decode_at::<S, R>(&Latent::new(self.controls_at::<S, R>(u, v, p)), p)
    }

    /// [`Chart::validate_params`] at the defaults.
    fn validate(&self, u: f64, v: f64) -> ValidationResult {
        self.validate_params(u, v, &LinkParams::DEFAULT)
    }

    /// The conjunction of the two axes' checks, each against its domain at the tunable parameters `p` (the module's
    /// note).
    fn validate_params(&self, u: f64, v: f64, p: &LinkParams<f64>) -> ValidationResult {
        let wide = |r: [P; 2]| [f64::from(r[0]), f64::from(r[1])];
        let checked = (
            axis_check(H::domain_at(p), wide(self.h_range), u),
            axis_check(V::domain_at(p), wide(self.v_range), v),
        );
        let failed = Constraint::AxisDomain;
        match checked {
            (Ok(cu), Ok(cv)) if cu == u && cv == v => ValidationResult::Valid,
            (Ok(cu), Ok(cv)) => ValidationResult::Clamp {
                u: cu,
                v: cv,
                moved: [cu != u, cv != v],
                failed,
            },
            _ => ValidationResult::Reject { failed },
        }
    }
}

impl<H: Quantity, V: Quantity, P: Copy> ChartDecode for Mixed<H, V, P>
where
    f64: From<P>,
{
    /// The chart's own [`Chart::decode`].
    #[inline]
    fn decode_at<S, R>(&self, u: R, v: R) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + WarpReal + From<P>,
    {
        Chart::decode::<S, R>(self, u, v)
    }

    /// The chart's own [`Chart::decode_params`].
    #[inline]
    fn decode_at_params<S, R>(&self, u: R, v: R, p: &LinkParams<R>) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + WarpReal + From<P>,
    {
        Chart::decode_params::<S, R>(self, u, v, p)
    }
}
