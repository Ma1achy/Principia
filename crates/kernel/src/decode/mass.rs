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
//! **The branch input** (REQ-VAL-017; dd_decoder §3.1, §5 test 12; gpu_determinism_note rules 1, 2 and 6; R-34 as
//! R-547 corrects it): `M₀₁` is the one rounded sum of the two masses the block emits, `round(m₀ + m₁)`, compared
//! strictly against the compile-time constant ε. There is no transcendental, division or `sqrt` at the decision, and
//! the formula's `M₀₁ < ε` and this form differ only where `|M₀₁ − ε|` is within that sum's half-ulp. It reads only the
//! emitted masses, never the link's intermediates, so [`from_masses`] gives a chart's simplex point the same test
//! (REQ-CHART-033).
//!
//! **Its form** (R-547, closing RQ-298; TASK-M2-48, REQ-DEC-011). A branch input that must agree across backends is
//! built only from operations every IEEE backend rounds alike, with no intermediate that can be subnormal. Written as
//! an add, `m₀ + m₁` is not that: with the link inlined, `stick_breaking`'s last step `m₁ = t₇·t₁₃` reaches a shader
//! compiler as `m₀ + t₇·t₁₃`, which a contracting backend may fuse, and at the onset crossings that flips the tag
//! (`decode_m01_tiny_contraction`: of 40 000 bisected crossing points, 77 at f64 and 45 at f32). The explicit
//! `fma(m₁, 1, m₀)` built until TASK-M2-48 is not that either: a backend may fold the `×1` and then contract the add
//! the same way (dd_decoder §3.1, "Its premise"), and an explicit `fma` is not fused on every backend (CI's lavapipe,
//! RQ-298). So `M₀₁` is formed in [`crate::canon`]'s `Wide` arithmetic, its pattern ([`crate::real::IeeeBits`]),
//! a mantissa and an integer exponent:
//! 1. **Each mass into the form** (`wide`): `mᵢ` scaled by `2^−k`, `k` its exponent field, as two normal powers of
//!    two, into `[1, 2)`. Each product is by a power of two with a normal result, so exact.
//! 2. **The sum** (`wide_add`): aligned on the larger exponent `e`, the smaller mantissa times `2^−s`, `s` the exponents'
//!    gap capped at 60, exact (a power of two, the result at least `2^−60`, normal); then one add of a value in
//!    `[1, 2)` and one in `[2^−60, 2)`, rounded once, in `[1, 4)`; then renormalised by a power of two, exact. Both
//!    terms were scaled by the same `2^−e` exactly, and scaling by a power of two commutes with rounding where nothing
//!    underflows or overflows, so that one rounding is `round(m₀ + m₁)·2^−e`: the one rounded sum, bit for bit. The
//!    cap moves the smaller term only where it is more than `2^60` below the larger, under half an ulp of it at either
//!    precision, where the sum rounds to the larger either way.
//! 3. **The decision** (`wide_greater`): `ε > M₀₁`, by exponent (integers), then by mantissa: comparisons only, against
//!    `ε` taken into the form exactly. [`Masses::m01`] is the sum converted to the float (`to_float`), exact where it
//!    is normal: a value the configuration block reads, not a branch input.
//!
//! So every product the add can meet is exact: a backend that fuses or contracts either of them into the add rounds
//! the same exact sum once, as one that does neither does, and one that folds a product by a power of two into it gets
//! the same exact value. No intermediate is subnormal, for normal masses: the scaled masses are in `[1, 2)` or at least
//! `2^−60`, the sum in `[1, 4)`, all normal at f32, so a backend that flushes subnormal results rounds as one that keeps
//! them. One pair of emitted masses therefore takes one branch on every IEEE backend that keeps its additions and
//! multiplications correctly rounded and unreassociated (fast-math off, R-84, R-297), whether it fuses, contracts or
//! flushes. A subnormal mass, below `2^−126` at f32, is read differently by a flushing backend before this block runs
//! (it emits 0 where the CPU keeps it), so the two backends' masses differ, and the cross-backend claim is for identical
//! inputs only (PIT-10): `M₀₁` is then compared on the masses each emitted (REQ-VAL-079; dd_decoder §3.1, "Its
//! premise").

use crate::canon::{to_float, wide, wide_add, wide_greater};
use crate::decode::degenerate::Degenerate;
use crate::generated::links::{Link, LinkParams, LinkReal, Selection};
use crate::real::IeeeBits;

/// The mass block's output: the simplex point, `M₀₁`, and the block's tag.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Masses<R> {
    /// `(m₀, m₁, m₂)`, `Σmᵢ = 1`.
    pub m: [R; 3],
    /// `M₀₁ = m₀ + m₁`, one rounding, formed in `Wide` arithmetic (the module's note) and converted to the float: the
    /// reconstruction's inner-pair mass. The tag reads the `Wide` sum it is converted from.
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

/// The mass block's output for the simplex point `m`, however it was produced: `M₀₁ = m₀ + m₁` and its tag. `M₀₁` is the
/// one rounded sum, formed in `Wide` arithmetic, and the tag is `ε > M₀₁` decided on it, so every IEEE backend that
/// keeps its additions and multiplications correctly rounded takes one branch on one pair of masses, fusing,
/// contracting or flushing or not (R-547; the module's note).
#[inline]
pub fn from_masses<R: LinkReal + IeeeBits>(m: [R; 3]) -> Masses<R> {
    let sum = wide_add(wide(m[0]), wide(m[1]));
    let degenerate = if wide_greater(wide(m01_epsilon::<R>()), sum) {
        Some(Degenerate::M01Tiny)
    } else {
        None
    };
    Masses {
        m,
        m01: to_float(sum),
        degenerate,
    }
}

/// `D_mass`: the mass controls `z[6:8]` through the selection's mass link, then [`from_masses`].
#[inline]
pub fn decode<S: Selection, R: LinkReal + IeeeBits>(z_mass: [R; 2]) -> Masses<R> {
    decode_at::<S, R>(z_mass, &LinkParams::DEFAULT)
}

/// [`decode`] at the tunable parameters `p`: the mass link reads the active `μ_max` (R-481 A2, G4; REQ-DEC-048).
#[inline]
pub fn decode_at<S: Selection, R: LinkReal + IeeeBits>(
    z_mass: [R; 2],
    p: &LinkParams<R>,
) -> Masses<R> {
    from_masses(<S::MassLink as Link<2, 3>>::forward_at(z_mass, p))
}
