//! A curved view's own tilt, its pivot record and its slider spans (chart_decoder_contract Part 4, "Tilt
//! (basis edit)" and "How the pin is held"; chart_reference §4.5, "The slider spans of the held coordinates"; render_gui_spec
//! § G2, "The slider span").
//!
//! - **The tilt a curved view holds** ([`CurvedTilt`]): the stored angles, the map, the targets and the targets' slider
//!   spans, by value, so a view is its own placement and Save view as chart reads it from the view. The family that edits
//!   it is [`super::Family::CurvedTilt`]; the kernel computes `R` from it ([`TiltSpec::columns`]).
//! - **The pivot record** ([`Pivot`]): where a lock is placed on a tilted curved view, the pinned state, as the chart's
//!   own coordinates (its chart point and hidden offsets, which with the chart decode it bit for bit), and the pin's screen
//!   position, its address in the window's `[0,1]²`, stored, not re-derived. Every later tilt and zoom turns about it: the
//!   pixel at the stored address is placed at the stored point with the stored offsets ([`super::Pivoted`], the kernel's
//!   tilt with the pivot's origin and offset).
//! - **The slider spans** ([`SliderSpan`]): fixed per chart from its home window, never read from the view, so
//!   a pan, a zoom, a slice or a lock leaves them as they are.

use kernel::chart::construction::{LzChart, RestChart, HELD_K, HELD_LZ, NU0};
use kernel::chart::invariant::WarpReal;
use kernel::chart::mixed::Quantity;
use kernel::chart::tilt::{Hidden, TiltMap, TiltSpec};
use kernel::chart::{
    BurrauThetaLz, HeldK, HeldLz, InvariantLE, InvariantLK, Mapped, Mapping, Mixed, MixedEnergy,
    MixedMassRatio, RestStart, ShapeSphere,
};
use ledger::constants::{
    BURRAU_THETA_LZ_GAMMA_K, BURRAU_THETA_LZ_K, EUCLID_NU_MAX, EUCLID_NU_MIN, LATENT_SLIDER_SPAN,
};
use serde::{Deserialize, Serialize};

/// A latent slider's span (render_gui_spec § G2): its rail `[−1, 1]`, the register's `latent_slider_span`.
pub const LATENT_SPAN: f64 = LATENT_SLIDER_SPAN.number();

/// `Φ_θ,L_z`'s held-`K` warp, `K_max`: the chart's calibrated `K`, the register's
/// `burrau_theta_lz_k`, so the preset's `t₀ = (K₀/K_max)^(1/γ_K)` is 1 exactly.
pub const HELD_K_K_MAX: f64 = BURRAU_THETA_LZ_K.number();

/// `Φ_θ,L_z`'s held-`K` warp, `γ_K`: the register's `burrau_theta_lz_gamma_k`, set by measurement
/// (`curved_view`'s `held_k_calibration`, its criterion written first).
pub const HELD_K_GAMMA_K: f64 = BURRAU_THETA_LZ_GAMMA_K.number();

/// `Φ_θ,L_z` holding its `K` under the calibrated warp ([`HELD_K_K_MAX`], [`HELD_K_GAMMA_K`]), warped, the default.
pub fn held_k(chart: BurrauThetaLz<f64>) -> HeldK<f64> {
    HeldK::new(chart, HELD_K_K_MAX, HELD_K_GAMMA_K)
}

/// The tilt a curved view holds: the stored angles in degrees, the map, the targets and their slider spans.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurvedTilt {
    /// `τ₁`, degrees.
    pub tau1: f64,
    /// `τ₂`, degrees.
    pub tau2: f64,
    /// Lean each axis where true, Tip as sheet where false.
    pub lean: bool,
    /// `τ₁`'s target.
    pub x: u32,
    /// `τ₂`'s target.
    pub y: u32,
    /// The targets' slider spans.
    pub span: [f64; 2],
}

impl CurvedTilt {
    /// The tilt `spec` at the spans `span`.
    pub fn of(spec: &TiltSpec, span: [f64; 2]) -> Self {
        CurvedTilt {
            tau1: spec.tau1,
            tau2: spec.tau2,
            lean: spec.map == TiltMap::Lean,
            x: spec.x,
            y: spec.y,
            span,
        }
    }

    /// The kernel's stored tilt.
    pub fn spec(&self) -> TiltSpec {
        TiltSpec {
            tau1: self.tau1,
            tau2: self.tau2,
            map: match self.lean {
                true => TiltMap::Lean,
                false => TiltMap::Sheet,
            },
            x: self.x,
            y: self.y,
        }
    }

    /// Whether `other` is this tilt at other angles: the same map, targets and spans, bit for bit.
    pub(super) fn same_axes(&self, other: &CurvedTilt) -> bool {
        self.lean == other.lean
            && (self.x, self.y) == (other.x, other.y)
            && self.span.map(f64::to_bits) == other.span.map(f64::to_bits)
    }

    /// This tilt with its angles `(τ₁, τ₂)`.
    pub(super) fn at(&self, [tau1, tau2]: [f64; 2]) -> Self {
        CurvedTilt {
            tau1,
            tau2,
            ..*self
        }
    }
}

/// The pivot record: the pinned state and the pin's screen position, held in a tilted curved view.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pivot {
    /// The pin's screen position, its address in the window's `[0,1]²`.
    pub address: [f64; 2],
    /// Whether the address was assumed at the centre because none could be derived; a click's is not.
    pub assumed: bool,
    /// The pinned state's chart point `(a, b)`.
    pub point: [f64; 2],
    /// The pinned state's hidden offsets `d0`, added to every pixel's offsets.
    pub d0: [f64; 2],
}

/// The slider span of a chart's hidden coordinate: a length, fixed per chart from its home window, that a tilt's hidden
/// offset is divided by. Never read from a view.
pub trait SliderSpan: Hidden {
    /// The span of the hidden coordinate `k`; `None` where `k` is not one of the chart's.
    fn slider_span(&self, k: u32) -> Option<f64>;
}

/// A latent control's span where `k` is hidden.
fn latent_span<C: Hidden>(chart: &C, k: u32) -> Option<f64> {
    chart.hidden(k).then_some(LATENT_SPAN)
}

impl SliderSpan for ShapeSphere<f64> {
    fn slider_span(&self, k: u32) -> Option<f64> {
        latent_span(self, k)
    }
}

impl SliderSpan for InvariantLE<f64> {
    fn slider_span(&self, k: u32) -> Option<f64> {
        latent_span(self, k)
    }
}

impl SliderSpan for InvariantLK<f64> {
    fn slider_span(&self, k: u32) -> Option<f64> {
        latent_span(self, k)
    }
}

impl<H: Quantity, V: Quantity> SliderSpan for Mixed<H, V, f64> {
    fn slider_span(&self, k: u32) -> Option<f64> {
        latent_span(self, k)
    }
}

impl<A, Q, S> SliderSpan for MixedEnergy<A, Q, S, f64>
where
    MixedEnergy<A, Q, S, f64>: Hidden,
{
    /// Its frozen latent controls'. Its held value is no hidden coordinate here: on a lone virial axis it has no
    /// chart-constant `K_max`, and its own span is not defined yet.
    fn slider_span(&self, k: u32) -> Option<f64> {
        latent_span(self, k)
    }
}

impl<A, S> SliderSpan for MixedMassRatio<A, S, f64>
where
    MixedMassRatio<A, S, f64>: Hidden,
{
    fn slider_span(&self, k: u32) -> Option<f64> {
        latent_span(self, k)
    }
}

impl<C, S> SliderSpan for Mapped<C, S>
where
    Mapped<C, S>: Hidden,
{
    /// The wrapped invariant chart's latent controls'.
    fn slider_span(&self, k: u32) -> Option<f64> {
        latent_span(self, k)
    }
}

/// `ν₀`'s span, the `ν` axis's range on the Euclid plane and the `(ν, K)` chart at home, `[1/32, 31/32]`: `15/16`.
pub const NU0_SPAN: f64 = EUCLID_NU_MAX.number() - EUCLID_NU_MIN.number();

impl<C: RestChart<Param = f64>> SliderSpan for RestStart<C, f64>
where
    RestStart<C, f64>: Hidden,
{
    /// `ν₀`'s [`NU0_SPAN`]; a momentum control's [`LATENT_SPAN`].
    fn slider_span(&self, k: u32) -> Option<f64> {
        match (self.hidden(k), k == NU0) {
            (false, _) => None,
            (true, true) => Some(NU0_SPAN),
            (true, false) => Some(LATENT_SPAN),
        }
    }
}

impl<C: LzChart<Param = P>, P: WarpReal> SliderSpan for HeldLz<C, P>
where
    HeldLz<C, P>: Hidden,
    f64: From<P>,
{
    /// The held `L_z`'s `2√(2·I·K_max)`, `I = 1`, `K_max` the chart's own `K` axis's: the `(L_z, K)` chart's top row.
    fn slider_span(&self, k: u32) -> Option<f64> {
        (self.hidden(k) && k == HELD_LZ).then(|| 2.0 * (2.0 * self.chart.k_max_at::<f64>()).sqrt())
    }
}

impl SliderSpan for HeldK<f64> {
    /// The held `K`'s: warped, 1 in its warp coordinate `t`; linear, `K_max` in `K`.
    fn slider_span(&self, k: u32) -> Option<f64> {
        (self.hidden(k) && k == HELD_K).then_some(match self.mapping {
            Mapping::Warped => 1.0,
            Mapping::Linear => self.k_max,
        })
    }
}
