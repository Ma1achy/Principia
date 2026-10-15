// The compactions of dd_colouring §3.6, payload scalar → b ∈ [0, 1], one form per ledger `scale` (colour_composition
// §1.2's Compaction; R-411): hand-written, following the generated prelude (`prelude.wgsl`) at assembly. `lin` is the
// prelude's own `range_norm` on a fixed range; this file holds the other four. Every form here is range-free but
// `lin` (R-411): `log` and `diverging` take a field's own scale, `cyclic` its period, `flag` nothing. The CPU mirror
// is `render::colour::field_ramp::Compaction::place`.
//
// Each form takes a finite value. A field ramp tests its value's bits first (R-114, R-436): an unset value, an
// invalid one and a non-finite one are styled before any form is called, so no form here does arithmetic on ±∞ or
// NaN, which WGSL may assume are absent (gpu_determinism_note rule 4).

// `log`: b = 1 − 1/(1 + ln(1 + |x|/ε)), the debug views' `dbg_log` placement (R-411), with the field's own ε > 0, its
// ledger `floor` (R-263) or its display floor (R-401). Linear below ε (b ≈ |x|/ε), logarithmic above it; 0 maps to
// b = 0 and |x| → ∞ to b → 1. x < 0, the stored −1.0 sentinel, maps to 0: the field ramp styles it before calling this
// (dd_colouring §5 test 7: the sentinel is styled, never ramped).
fn compact_log(x: f32, eps: f32) -> f32 {
    if (x < 0.0) {
        return 0.0;
    }
    let s = log(1.0 + abs(x) / eps);
    return 1.0 - 1.0 / (1.0 + s);
}

// `cyclic`: b = fract(x/period), WGSL's `fract`, x − ⌊x⌋, so b ∈ [0, 1) for a negative x too (phase, ρ_angle).
fn compact_cyclic(x: f32, period: f32) -> f32 {
    return fract(x / period);
}

// `diverging`, the symlog (R-411; the drift views' placement, R-381): b = ½ + ½·sign(x)·(1 − 1/(1 + ln(1 + |x|/x₀))),
// x₀ > 0 the field's own (dd_colouring §3.6, REQ-COL-065), no range and no x_max. The upper half `u ∈ [½, 1)` is
// formed from |x|, and a negative x takes `1 − u`, which is exact for u ∈ [½, 1] (Sterbenz), so b(x) + b(−x) = 1 holds
// exactly in f32, not only to rounding. x = 0, either sign, takes u = ½ by selection, not through `log(1)`, which a
// fast-math `log` need not return as exactly 0, so b(0) = ½ exactly.
fn compact_symlog(x: f32, x0: f32) -> f32 {
    let s = log(1.0 + abs(x) / x0);
    let u = select(0.5 + 0.5 * (1.0 - 1.0 / (1.0 + s)), 0.5, x == 0.0);
    return select(1.0 - u, u, x >= 0.0);
}

// `flag`: b ∈ {0, 1}, 1 where the value is set (non-zero).
fn compact_flag(x: f32) -> f32 {
    return select(0.0, 1.0, x != 0.0);
}
