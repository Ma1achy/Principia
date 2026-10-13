//! `D_mass`, the mass block (dd_decoder §3.1; chart_reference §0.1; REQ-DEC-010, REQ-DEC-011): the two mass controls
//! `z[6:8]` through the selection's mass link onto the simplex, `Σmᵢ = 1`. Under the default link, `softmax_tanh`,
//! that is `μₖ = μ_max·tanh(z_μk)` and `(m₀, m₁, m₂) = softmax(0, μ₁, μ₂)`, body 0 the softmax reference (logit 0),
//! so `z_μ = 0` gives equal masses. The block calls only the generated link (REQ-CHART-033): the formula is the
//! registry entry's forward, never re-derived here.
//!
//! **The one tag.** `M₀₁ = m₀ + m₁`; `DEGENERATE(M01_TINY)` iff `M₀₁ < ε`, and nothing else in the block tags
//! (REQ-DEC-011). A chart that sets masses directly (chart_reference §5.1's `ChartOut`: the ternary plot, the δm
//! strip) enters at [`from_masses`], which applies the same test, so the tag is the block's whatever produced the
//! simplex point.
//!
//! **ε = ε_M01 = 10⁻⁵** ([`m01_epsilon`]), the chart constant REQ-DEC-039's calibration proposes (R-71; dd_decoder
//! §3.1), provisional until the human confirms it at the M2 gate. It is a threshold set in a measured gap (pitfalls
//! §3), `fixtures/gates/m01_tiny/`: at `μ_max = 5` the reachable `M₀₁` floors are `stick_breaking`'s pile-up at
//! `ε_μ − ε_μ²/4` in the `m₂ → 1` corner, below ε; and the ternary plot's `2ε_m = 2×10⁻⁴` and `softmax_tanh`'s
//! `(1 + e^−μ_max)/(1 + e^−μ_max + e^μ_max) ≈ 6.74×10⁻³`, above it. So `stick_breaking`'s saturated corner fires with a
//! tenfold margin at both precisions, `softmax_tanh` and the ternary plot never fire, and the tag's onset lies where
//! `M₀₁` still changes with `z` at f32 resolution: the formula, not rounding, places it, and f64 and f32 agree on it to
//! about 10⁻⁷ in `z`. (At ε = ε_μ the onset sat 2 f32 ulps above the corner's floor, so f64 and f32, or two f32
//! backends' `tanh`, disagreed over a band of `z`.)
//!
//! **The branch input** (REQ-VAL-017; dd_decoder §3.1, §5 test 12; gpu_determinism_note rules 1, 2 and 6; R-34): `M₀₁`
//! is the one rounded sum of the two masses the block emits, compared strictly against the compile-time constant ε.
//! There is no transcendental, division or `sqrt` at the decision, and the formula's `M₀₁ < ε` and this form differ only
//! where `|M₀₁ − ε|` is within that sum's half-ulp. Written as an add, `m₀ + m₁` is not safe from contraction: with the
//! link inlined, `stick_breaking`'s last step `m₁ = t₇·t₁₃` reaches a shader compiler as `m₀ + t₇·t₁₃`, which a
//! contracting backend may fuse, and at the onset crossings that flips the tag (`decode_m01_tiny_contraction`: of 40 000
//! bisected crossing points, 77 at f64 and 45 at f32). So `M₀₁` is the explicit `fma(m₁, 1, m₀)`: `m₁·1` is exact, so it
//! is the same single rounding of `m₀ + m₁`, and no add is left to fuse. It reads only the emitted masses, never the
//! link's intermediates, so [`from_masses`] gives a chart's simplex point the same test (REQ-CHART-033).
//!
//! **Its premise** (dd_decoder §3.1): the form holds only on a backend that keeps an explicit `fma` fused and does not
//! fold the `×1`. Folding `fma(x, 1, y)` to `x + y` is exact and standard (LLVM's InstCombine, Mesa's NIR), and after it
//! a contracting backend can rebuild `fma(t₇, t₁₃, m₀)`. Whether the kernel's SPIR-V keeps GLSL.std.450 `Fma` fused
//! (qa's review observes it needs the `NoContraction` decoration) and whether rust-gpu emits that decoration is a check
//! for the second-backend parity run. `M₀₁` under `stick_breaking` at its onset crossings is a branch input for the
//! parity gate's no-fork test (R-34, REQ-VAL-017, REQ-VAL-079), which compares tags on identical emitted masses.

use crate::decode::degenerate::Degenerate;
use crate::generated::links::{Link, LinkReal, Selection};
use spirv_std::num_traits::Float;

/// The mass block's output: the simplex point, `M₀₁`, and the block's tag.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Masses<R> {
    /// `(m₀, m₁, m₂)`, `Σmᵢ = 1`.
    pub m: [R; 3],
    /// `M₀₁ = m₀ + m₁`, one rounding, formed as `fma(m₁, 1, m₀)`: the tag's branch input, and the reconstruction's
    /// inner-pair mass.
    pub m01: R,
    /// `Some(M01Tiny)` iff `M₀₁ < ε`; the block raises no other tag.
    pub degenerate: Option<Degenerate>,
}

/// The M01_TINY threshold ε: the chart constant ε_M01 (REQ-DEC-039's proposal, provisional until the M2 gate; the
/// module's note).
#[inline]
pub fn m01_epsilon<R: LinkReal>() -> R {
    R::EPS_M01
}

/// Whether the inner-pair mass `m01` is below ε: `m01 < ε`, strict, so `M₀₁ = ε` does not fire.
#[inline]
pub fn m01_tiny<R: LinkReal>(m01: R) -> bool {
    m01 < m01_epsilon::<R>()
}

/// The mass block's output for the simplex point `m`, however it was produced: `M₀₁ = m₀ + m₁` and its tag. `M₀₁` is
/// the explicit `fma(m₁, 1, m₀)` (R-34), which no backend contracts differently provided it keeps an explicit `fma`
/// fused and does not fold the `×1` (the module's note).
#[inline]
pub fn from_masses<R: LinkReal>(m: [R; 3]) -> Masses<R> {
    let m01 = Float::mul_add(m[1], R::one(), m[0]);
    let degenerate = if m01_tiny(m01) {
        Some(Degenerate::M01Tiny)
    } else {
        None
    };
    Masses { m, m01, degenerate }
}

/// `D_mass`: the mass controls `z[6:8]` through the selection's mass link, then [`from_masses`].
#[inline]
pub fn decode<S: Selection, R: LinkReal>(z_mass: [R; 2]) -> Masses<R> {
    from_masses(<S::MassLink as Link<2, 3>>::forward(z_mass))
}
