//! The chart system, the Question rung (systems_architecture §0; chart_decoder_contract Part 3; chart_reference §5.1):
//! a chart is a map `Φ : [0,1]² → Y` into a chart space, followed by the shared decoder `D` and canonicaliser `C`.
//! Every chart feeds the same `D ∘ C`, so adding a chart means adding a `Φ` and nothing else (REQ-CHART-003,
//! REQ-CHART-015): no chart module holds a mass, configuration or momentum decode formula, an integrator or a
//! renderer. [`decode()`] is `D ∘ Φ` through the shared [`mod@crate::decode`], whose output is the canonical `(m, r, p)`
//! for a chart that hands the decoder latent controls; a chart that sets the configuration explicitly ([`ShapeSphere`])
//! runs the same blocks through [`Chart::decode`], then `C`, so its output is canonical too. Every `D ∘ Φ` entry, this
//! one and `energy_norm::decode_chart`, goes through [`Chart::decode`].
//!
//! **The trait** ([`Chart`]; R-26, R-118): `map`, `Φ`, generic over the float type, so the one source is monomorphised
//! as the f32 kernel and the f64 CPU reference; `forbids_energy_normalisation` (R-25); `name`, which every dump header
//! carries ([`write_dump_header`]); and `validate(u, v) -> ValidationResult`, a CPU-side f64 check
//! (inverse_encode_contract § "Chart-aware validation"), whose variants chart_reference §5.1 defines (R-72,
//! REQ-CHART-043).
//!
//! **The charts, in chart_reference §5.1's order** (REQ-CHART-029): [`Latent`] (`latent`), then [`ShapeSphere`]
//! (`shape_sphere`), [`Burrau`] and the [`EuclidPlane`] (`burrau`) with the acute-angle charts [`BurrauThetaK`],
//! [`BurrauThetaLz`] and [`BurrauThetaDeltaM`] (`burrau_acute`), [`InvariantLE`] / [`InvariantLK`] (`invariant`) and
//! `MassSimplex`, each a `Φ` beside it.
//!
//! **Axis metadata and the statistics guard** ([`AxisMeta`], [`admit_statistic`]; chart_decoder_contract Part 5): an
//! axis declares the decoder blocks it writes and whether it is annotation-only. A chart with an annotation axis is a
//! 1-swept family rendered in 2D ([`swept`], [`family_label`]): it supports per-value statistics along its swept axis
//! only, and a 2D density claim across the annotation axis is refused.
//!
//! **`system_image` and the shape-fraction guard** ([`SystemImage`], [`ChartLabel`], [`admit_shape_fraction`];
//! chart_decoder_contract Part 5; chart_reference §4.5; R-27, R-104, R-157): a chart's descriptor of how its pixels
//! fold onto distinct systems, one of the contract's four values, shown in the chart's label with the quotient it
//! covers. A shape fraction is refused on a `DoubleCover` chart, which counts each shape twice (R-27), and on a
//! ray-degenerate one, whose areas are not system fractions. The Burrau charts carry it (`burrau`, `burrau_acute`).
//!
//! **Each chart's decode** ([`ChartDecode`]): `D ∘ Φ` through the shared blocks. A chart whose `Φ` hands `D` only `z`
//! decodes by [`decode()`]; an invariant chart hands the construction its targets `(L_z, K*)` in place of the free
//! momentum block ([`invariant::decode`]). The decode stage is generic over this trait, so each chart's own decode is
//! what the kernel monomorphises.
//!
//! **The chart's parameters** are held at a float type of their own, [`Chart::Param`]: f32 in the kernel's uniform, f64
//! on the CPU. `map` is generic over any float type the parameters widen into (`From<Self::Param>`), so a chart held at
//! f32 maps at f32 on the GPU and one held at f64 maps at f64 on the CPU; narrowing f64 parameters to f32 is the CPU's
//! own step, never `map`'s.

pub mod burrau;
pub mod burrau_acute;
pub mod invariant;
pub mod latent;
pub mod shape_sphere;

use core::fmt;

use spirv_std::num_traits::Float;

pub use burrau::{Burrau, EuclidPlane};
pub use burrau_acute::{BurrauThetaDeltaM, BurrauThetaK, BurrauThetaLz};
pub use invariant::{InvariantLE, InvariantLK};
pub use latent::Latent;
pub use shape_sphere::{ShapeMap, ShapeSphere};

use crate::decode::{self, Decoded};
use crate::generated::links::{LinkReal, Selection};
use crate::payload::PayloadReal;

/// What `Φ` hands the shared decoder (chart_reference §5.1: `ChartOut` is whatever `D` consumes). The latent chart
/// hands it the eight controls `z`; a chart that sets masses, `(α, β)`, an explicit `(ρ, λ)` or a `(L_z, K)` request
/// adds its output here with the chart.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChartOut<F> {
    /// The latent controls `z`, in the decoder's block layout.
    pub z: decode::Latent<F>,
}

/// The constraint a [`ValidationResult`] names as failed (chart_reference §5.1; inverse_encode_contract § "Chart-aware
/// validation", layers 2 and 3, in that order). Layer 1, the hypercube bounds, is the encode path's on lookup and lock,
/// never `validate`'s (chart_decoder_contract Part 3, R-407).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Constraint {
    /// Layer 2, `(L_z, E)`: inside the parabola `|L_z| ≤ √(2I(E − U))`.
    LzEParabola,
    /// Layer 2, `(L_z, K)`: `K ≥ 0`.
    KNonNegative,
    /// Layer 2, `(L_z, K)`: `K ≥ L_z²/2I`.
    KAboveLzBound,
    /// Layer 2, mass simplex: every `mᵢ > 0`.
    MassPositive,
    /// Layer 2, mass simplex: `Σmᵢ = 1`.
    MassSum,
    /// Layer 2, mass simplex: the interior buffer `ε_m`, `mᵢ ≥ ε_m`.
    MassBuffer,
    /// Layer 2, shape sphere: the azimuth `θ ∈ [0, 2π)`.
    ShapeTheta,
    /// Layer 2, shape sphere: the polar angle `φ ∈ [0, π]`, no polar buffer.
    ShapePhi,
    /// Layer 2, Burrau charts: `ν ∈ (0, 1)`.
    BurrauNu,
    /// Layer 2, the acute-angle Burrau charts: `θ ∈ (0, π/4]`.
    BurrauTheta,
    /// Layer 3, decode sanity: every decoded mass positive.
    DecodedMassPositive,
    /// Layer 3, decode sanity: the CoM at the origin, within tolerance.
    ComAtOrigin,
    /// Layer 3, decode sanity: the total momentum zero, within tolerance.
    MomentumZero,
}

/// What [`Chart::validate`] returns for a chart point `(u, v)` (chart_reference §5.1; R-26). On a failure the chart
/// gives the first preferred remedy that keeps the IC the one intended: project, then clamp, then reject
/// (inverse_encode_contract § "Chart-aware validation", "On failure").
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ValidationResult {
    /// `(u, v)` passes every layer-2 and layer-3 constraint of the chart.
    Valid,
    /// `(u, v)` fails `failed`; `(u, v)` here is the nearest point of the chart's feasible region, where lookup and
    /// lock commit, and notify.
    Project { u: f64, v: f64, failed: Constraint },
    /// `(u, v)` fails `failed`; each out-of-range coordinate is snapped to its boundary, `(u, v)` here, and `moved`
    /// says which of `u`, `v` moved, for `lookup_clamped` and the display of the moved coordinates.
    Clamp {
        u: f64,
        v: f64,
        moved: [bool; 2],
        failed: Constraint,
    },
    /// `(u, v)` is refused, with an error naming `failed`: projecting or clamping would give a qualitatively different
    /// IC from the one intended.
    Reject { failed: Constraint },
}

/// A chart (chart_reference §5.1; R-26, R-118).
pub trait Chart {
    /// The float type the chart's parameters are held at: f32 in the kernel, f64 on the CPU.
    type Param: Copy;

    /// `Φ : [0,1]² → chart space`, generic over the float type (R-118): instantiated at f32 by the kernel and at f64 on
    /// the CPU, from this one source.
    fn map<F: Float + From<Self::Param>>(&self, u: F, v: F) -> ChartOut<F>;

    /// Whether the chart forbids energy normalisation (chart_reference §0.6; R-25): `true` where energy is a chart
    /// coordinate or is enforced by the momentum construction.
    fn forbids_energy_normalisation(&self) -> bool {
        false
    }

    /// The chart's name, written in every dump header ([`write_dump_header`]).
    fn name(&self) -> &str;

    /// The chart's domain function (R-26), a CPU-side f64 check: inverse_encode_contract § "Chart-aware validation"'s
    /// layers 2 and 3 at `(u, v)`.
    fn validate(&self, u: f64, v: f64) -> ValidationResult;

    /// `D ∘ Φ` at `(u, v)` under the link selection `S`, through the shared decoder. By default the latent controls
    /// `map` hands over, decoded by the shared [`mod@crate::decode`]; a chart whose output those controls cannot carry,
    /// an explicit configuration, overrides it with the same shared blocks ([`ShapeSphere`]).
    #[inline]
    fn decode<S, R>(&self, u: R, v: R) -> Decoded<R>
    where
        Self: Sized,
        S: Selection,
        R: LinkReal + PayloadReal + From<Self::Param>,
    {
        decode::decode::<S, R>(&self.map(u, v).z)
    }
}

/// `D ∘ Φ` at `(u, v)` under the link selection `S`: the chart's output through the shared decoder, which no chart
/// re-implements (REQ-CHART-015).
#[inline]
pub fn decode<S, R, C>(chart: &C, u: R, v: R) -> Decoded<R>
where
    S: Selection,
    R: LinkReal + PayloadReal + From<C::Param>,
    C: Chart,
{
    chart.decode::<S, R>(u, v)
}

/// A chart's decode, `D ∘ Φ` at `(u, v)` under the link selection `S`, through the shared blocks (REQ-CHART-015): the
/// decode stage's ([`crate::stage::decode::sample`]).
pub trait ChartDecode: Chart {
    /// The chart's decode at `(u, v)`.
    fn decode_at<S, R>(&self, u: R, v: R) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + invariant::WarpReal + From<Self::Param>;
}

impl<P: Copy> ChartDecode for Latent<P> {
    /// [`decode()`]: the shared decoder on `Φ`'s `z`.
    #[inline]
    fn decode_at<S, R>(&self, u: R, v: R) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + invariant::WarpReal + From<P>,
    {
        decode::<S, R, Self>(self, u, v)
    }
}

impl<P: Copy> ChartDecode for InvariantLE<P> {
    /// [`invariant::decode`]: the construction's outcome is not part of [`Decoded`] (`invariant`'s note).
    #[inline]
    fn decode_at<S, R>(&self, u: R, v: R) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + invariant::WarpReal + From<P>,
    {
        invariant::decode::<S, R, Self>(self, u, v).0
    }
}

impl<P: Copy> ChartDecode for InvariantLK<P> {
    /// [`invariant::decode`], as for [`InvariantLE`].
    #[inline]
    fn decode_at<S, R>(&self, u: R, v: R) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + invariant::WarpReal + From<P>,
    {
        invariant::decode::<S, R, Self>(self, u, v).0
    }
}

impl<P: Copy> ChartDecode for Burrau<P>
where
    f64: From<P>,
{
    /// [`burrau::decode`]: the construction's outcome is not part of [`Decoded`], as for the invariant charts.
    #[inline]
    fn decode_at<S, R>(&self, u: R, v: R) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + invariant::WarpReal + From<P>,
    {
        burrau::decode(self, u, v).0
    }
}

impl<P: Copy> ChartDecode for EuclidPlane<P>
where
    f64: From<P>,
{
    /// The chart's own [`Chart::decode`], [`burrau::decode_plane`]: the Burrau system at `ν(v)`, at rest; `u` is not
    /// read.
    #[inline]
    fn decode_at<S, R>(&self, u: R, v: R) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + invariant::WarpReal + From<P>,
    {
        Chart::decode::<S, R>(self, u, v)
    }
}

/// The decoder blocks an axis writes, its block-touch set (chart_decoder_contract Part 5): empty for an annotation axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockTouch {
    /// The configuration block.
    pub config: bool,
    /// The momentum block.
    pub momentum: bool,
    /// The mass block.
    pub mass: bool,
}

impl BlockTouch {
    /// Whether the set is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        !(self.config | self.momentum | self.mass)
    }
}

/// The per-axis metadata the validator and the statistics read (chart_decoder_contract Part 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AxisMeta {
    /// The blocks the axis writes.
    pub touches: BlockTouch,
    /// Annotation-only: the axis drives no IC degree of freedom, a display and landmark coordinate the decoder ignores.
    pub annotation: bool,
}

/// One of a chart's two axes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    /// The horizontal axis, `u`.
    Horizontal,
    /// The vertical axis, `v`.
    Vertical,
}

/// A quantitative statistic asked of a chart's pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Statistic {
    /// A per-value statistic along one axis: at each value of that axis, a statistic over the pixels there.
    PerValue(Axis),
    /// A 2D density claim: an area fraction or density over both axes of the chart.
    Density2D,
}

/// Why [`admit_statistic`] refused a statistic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatisticRefused {
    /// A 2D density across an annotation axis, which drives no degree of freedom (chart_decoder_contract Part 5).
    DensityAcrossAnnotation,
    /// A per-value statistic along an annotation axis: every value of it is the same family.
    AlongAnnotation,
    /// A shape fraction on a `DoubleCover` chart, which covers each shape twice, as two labelled systems (R-27).
    ShapeFractionOnDoubleCover,
    /// A shape fraction on a ray-degenerate chart, whose areas are not system fractions (chart_decoder_contract Part 5).
    ShapeFractionOnRayDegenerate,
}

/// The metadata of `axis` among `axes`, `[horizontal, vertical]`.
#[inline]
fn meta(axes: [AxisMeta; 2], axis: Axis) -> AxisMeta {
    match axis {
        Axis::Horizontal => axes[0],
        Axis::Vertical => axes[1],
    }
}

/// The statistics guard (chart_decoder_contract Part 5; REQ-CHART-013): on a chart with an annotation axis, only
/// per-value statistics along a swept axis are admitted; a 2D density, and a per-value statistic along the annotation
/// axis, are refused. A chart with no annotation axis admits both.
pub fn admit_statistic(axes: [AxisMeta; 2], statistic: Statistic) -> Result<(), StatisticRefused> {
    match statistic {
        Statistic::Density2D if axes[0].annotation | axes[1].annotation => {
            Err(StatisticRefused::DensityAcrossAnnotation)
        }
        Statistic::PerValue(axis) if meta(axes, axis).annotation => {
            Err(StatisticRefused::AlongAnnotation)
        }
        _ => Ok(()),
    }
}

/// How many of the chart's axes are swept, not annotation-only: 2 for a 2-swept chart, 1 for a 1-swept family rendered
/// in 2D (chart_decoder_contract Part 5).
pub fn swept(axes: [AxisMeta; 2]) -> u32 {
    (!axes[0].annotation) as u32 + (!axes[1].annotation) as u32
}

/// The label a chart with these axes carries for its well-posedness (chart_decoder_contract Part 5: a chart with an
/// annotation axis "must be labelled as such").
pub fn family_label(axes: [AxisMeta; 2]) -> &'static str {
    match swept(axes) {
        2 => "2-swept chart",
        1 => "1-swept family rendered in 2D",
        _ => "no swept axis",
    }
}

/// How a chart's address map folds onto distinct systems: the `system_image` descriptor (chart_decoder_contract Part 5,
/// its four values; R-104, R-141, R-157).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemImage {
    /// Every pixel is a distinct system.
    Bijective,
    /// A fixed finite number `n` of pixels share each system; the shape sphere is 2-to-1 over its φ hemispheres (R-141).
    NToOne(u32),
    /// Each shape is covered twice, as two labelled systems: the full-range Burrau chart, where the leg swap relabels
    /// the bodies (R-27, R-104, R-157).
    DoubleCover,
    /// Whole lines of pixels map to the same system.
    RayDegenerate,
}

impl fmt::Display for SystemImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SystemImage::Bijective => f.write_str("bijective"),
            SystemImage::NToOne(n) => write!(f, "{n}-to-1"),
            SystemImage::DoubleCover => f.write_str("DoubleCover"),
            SystemImage::RayDegenerate => f.write_str("ray-degenerate"),
        }
    }
}

/// A chart's label (chart_reference §4.5, R-27: "label each chart with the quotient it covers"): its name, its
/// [`SystemImage`] and, in words, the quotient that value covers. It displays as `<name> · system_image <value> ·
/// <quotient>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChartLabel<'a> {
    /// The chart's [`Chart::name`].
    pub name: &'a str,
    /// The chart's `system_image`.
    pub image: SystemImage,
    /// The quotient the chart covers, in words.
    pub quotient: &'static str,
}

impl fmt::Display for ChartLabel<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} · system_image {} · {}",
            self.name, self.image, self.quotient
        )
    }
}

/// The shape-fraction guard (chart_reference §4.5; R-27; chart_decoder_contract Part 5): a fraction of shapes taken
/// over a chart's pixels is refused on a [`SystemImage::DoubleCover`] chart, which counts each shape twice, and on a
/// [`SystemImage::RayDegenerate`] one, whose areas are not system fractions; a bijective or n-to-1 chart admits it.
pub fn admit_shape_fraction(image: SystemImage) -> Result<(), StatisticRefused> {
    match image {
        SystemImage::DoubleCover => Err(StatisticRefused::ShapeFractionOnDoubleCover),
        SystemImage::RayDegenerate => Err(StatisticRefused::ShapeFractionOnRayDegenerate),
        SystemImage::Bijective | SystemImage::NToOne(_) => Ok(()),
    }
}

/// Writes a dump's header, which stamps the chart's [`Chart::name`] (chart_reference §5.1): the line
/// `chart: <name>`.
pub fn write_dump_header<W: fmt::Write, C: Chart>(out: &mut W, chart: &C) -> fmt::Result {
    writeln!(out, "chart: {}", chart.name())
}
