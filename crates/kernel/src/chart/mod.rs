//! The chart system, the Question rung (systems_architecture §0; chart_decoder_contract Part 3; chart_reference §5.1):
//! a chart is a map `Φ : [0,1]² → Y` into a chart space, followed by the shared decoder `D` and canonicaliser `C`.
//! Every chart feeds the same `D ∘ C`, so adding a chart means adding a `Φ` and nothing else (REQ-CHART-003,
//! REQ-CHART-015): no chart module holds a mass, configuration or momentum decode formula, an integrator or a
//! renderer. [`decode()`] is `D ∘ Φ` through the shared [`mod@crate::decode`], whose output is the canonical `(m, r, p)`.
//!
//! **The trait** ([`Chart`]; R-26, R-118): `map`, `Φ`, generic over the float type, so the one source is monomorphised
//! as the f32 kernel and the f64 CPU reference; `forbids_energy_normalisation` (R-25); `name`, which every dump header
//! carries ([`write_dump_header`]); and `validate(u, v) -> ValidationResult`, a CPU-side f64 check
//! (inverse_encode_contract § "Chart-aware validation"), whose variants chart_reference §5.1 defines (R-72,
//! REQ-CHART-043).
//!
//! **The charts, in chart_reference §5.1's order** (REQ-CHART-029): [`Latent`] (`latent`), then `ShapeSphere`,
//! `Burrau`, [`InvariantLE`] / [`InvariantLK`] (`invariant`) and `MassSimplex`, each a `Φ` beside it.
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

pub mod invariant;
pub mod latent;

use core::fmt;

/// The float trait [`Chart::map`] is generic over (R-118), re-exported so that a chart written outside the kernel, a
/// test's among them, names the same trait (TASK-M2-32).
pub use spirv_std::num_traits::Float;

pub use invariant::{InvariantLE, InvariantLK};
pub use latent::Latent;

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
    decode::decode::<S, R>(&chart.map(u, v).z)
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

/// Writes a dump's header, which stamps the chart's [`Chart::name`] (chart_reference §5.1): the line
/// `chart: <name>`.
pub fn write_dump_header<W: fmt::Write, C: Chart>(out: &mut W, chart: &C) -> fmt::Result {
    writeln!(out, "chart: {}", chart.name())
}
