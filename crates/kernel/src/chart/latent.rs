//! `Latent { z0, q1, q2 }`, the latent chart (chart_reference §1.1, §5.1; chart_decoder_contract Part 3; R-83): the
//! affine slice
//!
//! ```text
//! z(u, v) = z₀ + (2u − 1)·q₁ + (2v − 1)·q₂
//! ```
//!
//! through the 8D latent space, `z₀` the slice centre and `q₁, q₂` the two directions spanning the plane. The
//! `(2u − 1), (2v − 1)` factors place the unsigned addressing space `[0,1]²` into the signed plane centred on `z₀`
//! (coordinate note, "The three coordinate spaces"). It is the general case: an axis-aligned slice (`q₁ = ê_i`,
//! `q₂ = ê_j`) and an oblique one (any orthonormal pair) are both instances.
//!
//! **The scale lives in `q`** (R-83): zoom is a common scale on `q₁` and `q₂` ([`Latent::zoomed`]), and the chart has
//! no per-axis factor; its fields are `z0`, `q1` and `q2` and nothing else.
//!
//! **No decode here** (REQ-CHART-015): `Φ` only places `z`; the masses, configuration and momenta come from the
//! shared decoder ([`fn@crate::chart::decode`]).
//!
//! **Validation** (R-26): the latent chart adds no constraint beyond the hypercube (inverse_encode_contract §
//! "Chart-aware validation", layer 2 lists none for it), and layer 1 is the encode path's, never a pixel's
//! (chart_decoder_contract Part 3, R-407), so its layer-2 check accepts every `(u, v)`. Layer 3, decode sanity, holds
//! by the shared decoder's construction (chart_reference §5.1): the masses are positive under every registered mass
//! link (§0.1), the CoM is at the origin (§0.2) and `Σpᵢ = 0` (§0.3), the residuals rounding within REQ-ENC-027's
//! tolerance. So `validate` is `Valid` at every `(u, v)`. Both axes are affine past `[0,1]²` (chart_reference §5.4):
//! the formula continues.

use spirv_std::num_traits::Float;

use super::{Chart, ChartOut, ValidationResult};
use crate::decode;

/// The latent chart: slice centre `z0` and the plane's directions `q1`, `q2`, each in the decoder's block layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Latent<P> {
    /// The slice centre `z₀`, at `(u, v) = (½, ½)`.
    pub z0: [P; 8],
    /// The horizontal direction `q₁`.
    pub q1: [P; 8],
    /// The vertical direction `q₂`.
    pub q2: [P; 8],
}

impl<P: Float> Latent<P> {
    /// The chart zoomed by `k` about its centre (R-83): `q₁` and `q₂` scaled by the one common factor `k`, `z₀` kept.
    #[inline]
    pub fn zoomed(&self, k: P) -> Self {
        Latent {
            z0: self.z0,
            q1: self.q1.map(|q| q * k),
            q2: self.q2.map(|q| q * k),
        }
    }
}

/// One component of `z₀ + a·q₁ + b·q₂`, from that component of `z₀`, `q₁` and `q₂`.
#[inline]
fn place<P, F: Float + From<P>>(z0: P, q1: P, q2: P, a: F, b: F) -> F {
    let wide = <F as From<P>>::from;
    wide(z0) + a * wide(q1) + b * wide(q2)
}

impl<P: Copy> Chart for Latent<P> {
    type Param = P;

    /// `z(u, v) = z₀ + (2u − 1)·q₁ + (2v − 1)·q₂`, each component read by a literal index in this source, so no
    /// runtime index reaches the GPU build (gpu_determinism_note rule 5).
    #[inline]
    fn map<F: Float + From<P>>(&self, u: F, v: F) -> ChartOut<F> {
        let a = u + u - F::one();
        let b = v + v - F::one();
        ChartOut {
            z: decode::Latent::new([
                place(self.z0[0], self.q1[0], self.q2[0], a, b),
                place(self.z0[1], self.q1[1], self.q2[1], a, b),
                place(self.z0[2], self.q1[2], self.q2[2], a, b),
                place(self.z0[3], self.q1[3], self.q2[3], a, b),
                place(self.z0[4], self.q1[4], self.q2[4], a, b),
                place(self.z0[5], self.q1[5], self.q2[5], a, b),
                place(self.z0[6], self.q1[6], self.q2[6], a, b),
                place(self.z0[7], self.q1[7], self.q2[7], a, b),
            ]),
        }
    }

    fn name(&self) -> &str {
        "Latent"
    }

    /// `true`: `q₁` or `q₂` may sweep the mass controls, and the chart's type does not say whether they do, so no uniform
    /// mass is trusted.
    fn requires_per_pixel_mass(&self) -> bool {
        true
    }

    /// [`ValidationResult::Valid`] at every `(u, v)`: the chart has no layer-2 constraint, and layer 3 holds by the
    /// shared decoder's construction (the module's header).
    fn validate(&self, _u: f64, _v: f64) -> ValidationResult {
        ValidationResult::Valid
    }
}
