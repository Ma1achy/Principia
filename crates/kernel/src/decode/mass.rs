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
//! **ε = ε_μ** ([`m01_epsilon`]), the proposal of REQ-DEC-039's calibration (R-71), provisional until the human
//! confirms it at the M2 gate. Its evidence, `fixtures/gates/m01_tiny/`: at `μ_max = 5`, `softmax_tanh` keeps
//! `M₀₁ ≥ (1 + e^−μ_max)/(1 + e^−μ_max + e^μ_max) ≈ 6.74×10⁻³` for every finite `z`, so it never fires there;
//! `stick_breaking` reaches `M₀₁ → ε_μ − ε_μ²/4` in the `m₂ → 1` corner, so it fires there, where both controls sit in
//! the link's `ε_μ/2` edge buffers; the ternary plot keeps `M₀₁ ≥ 2ε_m = 2×10⁻⁴` inside `[0,1]²`. ε_μ is the mass
//! links' own resolution constant, and the tag marks exactly the inner-pair masses below it.
//!
//! **The branch input** (REQ-VAL-017; dd_decoder §5 test 12; gpu_determinism_note rules 1, 2 and 6): `M₀₁` is one
//! IEEE add of the two masses the block emits, compared strictly against the compile-time constant ε: no
//! transcendental, division, `sqrt` or contractible multiply-add at the decision. So identical masses give the
//! identical tag on every backend, and the tag agrees with the emitted masses bit for bit; the formula's `M₀₁ < ε`
//! and this form differ only where `|M₀₁ − ε|` is within that add's half-ulp.

use crate::decode::degenerate::Degenerate;
use crate::generated::links::{Link, LinkReal, Selection};

/// The mass block's output: the simplex point, `M₀₁`, and the block's tag.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Masses<R> {
    /// `(m₀, m₁, m₂)`, `Σmᵢ = 1`.
    pub m: [R; 3],
    /// `M₀₁ = m₀ + m₁`, one rounded add: the tag's branch input, and the reconstruction's inner-pair mass.
    pub m01: R,
    /// `Some(M01Tiny)` iff `M₀₁ < ε`; the block raises no other tag.
    pub degenerate: Option<Degenerate>,
}

/// The M01_TINY threshold ε: ε_μ (REQ-DEC-039's proposal, provisional until the M2 gate; the module's note).
#[inline]
pub fn m01_epsilon<R: LinkReal>() -> R {
    R::EPS_MU
}

/// Whether the inner-pair mass `m01` is below ε: `m01 < ε`, strict, so `M₀₁ = ε` does not fire.
#[inline]
pub fn m01_tiny<R: LinkReal>(m01: R) -> bool {
    m01 < m01_epsilon::<R>()
}

/// The mass block's output for the simplex point `m`, however it was produced: `M₀₁ = m₀ + m₁` and its tag.
#[inline]
pub fn from_masses<R: LinkReal>(m: [R; 3]) -> Masses<R> {
    let m01 = m[0] + m[1];
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
