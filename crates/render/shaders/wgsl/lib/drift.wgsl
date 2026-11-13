// The drift views' presentation (render contract Part 5, "Presentation layer"; R-379, R-381; REQ-TOOL-149;
// TASK-M3-05): hand-written, part of the presentation layer, in a file of its own because it calls the field ramp's
// compactions and ramps (`compaction.wgsl`'s `log1p_ratio`, `ramp.wgsl`'s `ramp_diverging`), which follow the
// presentation layer at assembly, as `present.wgsl`'s helpers call only the prelude's. The energy-drift and L_z-drift
// views draw the current drift `x` through `dbg_drift`, then the drift suspect predicates' styling through
// `dbg_suspect`. The CPU mirror is `crate::present::{dbg_drift, dbg_suspect}`.

// The drift views' compactions, the `COMPACTION` uniform's values (R-381): `symlog`, the default, `lin` and `log`.
const DBG_DRIFT_SYMLOG: u32 = 0u;
const DBG_DRIFT_LIN: u32 = 1u;
const DBG_DRIFT_LOG: u32 = 2u;

// The drift views' `diverging(c−, c0, c+)` ramp: Moreland's cool-warm ends and neutral, c− #3B4CC0, c0 #DDDDDD and
// c+ #B40426, 8-bit sRGB (R-72), mixed in linear RGB by `ramp_diverging`; or viridis, `dbg_sentinel`'s ramp, under the
// palette swap (`viridis` true; colour_composition §6). The CPU mirror is `crate::present::DRIFT_DIVERGING_SRGB8`.
fn dbg_drift_ramp(t: f32, viridis: bool) -> vec3<f32> {
    let lo = dbg_srgb8(vec3<u32>(59u, 76u, 192u));
    let mid = dbg_srgb8(vec3<u32>(221u, 221u, 221u));
    let hi = dbg_srgb8(vec3<u32>(180u, 4u, 38u));
    return select(ramp_diverging(t, lo, mid, hi), ramp_viridis(t), viridis);
}

// A drift's place `t` on its ramp, by its compaction (R-381), `x` first clamped to ±1e30 as `dbg_literal` clamps it.
// `symlog`: the compacted value `s = sign(x)·ln(1 + |x|/eps)` at `dbg_sentinel`'s place, `t = 0.5 + 0.5·s/(1 + |s|)`
// (`dbg_literal`), 0 at the middle. `lin`: `range_norm(x, −lin_r, lin_r, false, ·)`, the fixed range, clamped, so a
// drift beyond ±lin_r is at an end. `log`: the magnitude `|x|` at `dbg_log`'s place, `1 − 1/(1 + ln(1 + |x|/eps))`,
// so a loss and a gain of one size share a place. `eps > 0` is the field's floor, `ε_E` or `ε_L`; `ln(1 + a/eps)` is
// `log1p_ratio`'s, which never forms an overflowing quotient. Any other compaction is `symlog`.
fn dbg_drift_place(x: f32, compaction: u32, lin_r: f32, eps: f32) -> f32 {
    let c = clamp(x, -1e30, 1e30);
    if (compaction == DBG_DRIFT_LIN) {
        return range_norm(c, -lin_r, lin_r, false, vec2<f32>(0.0, 0.0));
    }
    let s = log1p_ratio(abs(c), eps);
    if (compaction == DBG_DRIFT_LOG) {
        return 1.0 - 1.0 / (1.0 + s);
    }
    return dbg_literal(select(-s, s, c >= 0.0));
}

// The neutral band's factors, `k'·2⁻²³` (render contract Part 5, "The neutral band"; R-928): the energy's `k' = 15`,
// so `15·2⁻²³`, of `K + |V|`, and `L_z`'s `k' = 5`, so `5·2⁻²³`, of `S = Σ(|xᵢp_{y,i}| + |yᵢp_{x,i}|)`, each `k'` the
// rounding bound of a fresh sample's drift against a CPU reference. The CPU mirrors are
// `crate::present::{DRIFT_BAND_E, DRIFT_BAND_LZ}`.
const DBG_DRIFT_BAND_E: f32 = 1.7881393432617188e-6;
const DBG_DRIFT_BAND_LZ: f32 = 5.960464477539062e-7;

// Whether `x` is NaN or `±∞`, an exponent of all ones, tested on its bits before any arithmetic (R-436; render_gui_spec
// §10.1; gpu_determinism_note rule 4). WGSL pins neither the bits of `∞ − ∞` nor a comparison on `∞`, so a drift view
// tests its reference and its drift by this, and draws the hatch, before it reads either as a number. The CPU mirror is
// `ledger::gen::numeric::non_finite`.
fn dbg_drift_non_finite(x: f32) -> bool {
    return (bitcast<u32>(x) & 0x7f800000u) == 0x7f800000u;
}

// The rounding's own scale for the energy, `K + |V|`, `K = Σᵢ ‖pᵢ‖²/2mᵢ` and `|V| = Σ_{i<j} mᵢmⱼ/‖rᵢ − rⱼ‖`, the
// masses `m` the sample's `ICDescriptor`'s: the sizes of the terms `H(r, p) − E_0` sums, whatever `E_0` is (R-928). The
// CPU mirror is `crate::present::drift_rounding_energy`.
fn dbg_drift_rounding_energy(r: array<vec2<f32>, 3>, p: array<vec2<f32>, 3>, m: vec3<f32>) -> f32 {
    let r01 = r[0] - r[1];
    let r02 = r[0] - r[2];
    let r12 = r[1] - r[2];
    let k = dot(p[0], p[0]) / (2.0 * m.x) + dot(p[1], p[1]) / (2.0 * m.y) + dot(p[2], p[2]) / (2.0 * m.z);
    let v = m.x * m.y / sqrt(dot(r01, r01)) + m.x * m.z / sqrt(dot(r02, r02)) + m.y * m.z / sqrt(dot(r12, r12));
    return k + v;
}

// The rounding's own scale for `L_z`, `S = Σᵢ (|xᵢp_{y,i}| + |yᵢp_{x,i}|)`: the sizes of the six products
// `L_z(r, p) − Lz_0` sums, whatever `Lz_0` is (R-928). The CPU mirror is `crate::present::drift_rounding_lz`.
fn dbg_drift_rounding_lz(r: array<vec2<f32>, 3>, p: array<vec2<f32>, 3>) -> f32 {
    return (abs(r[0].x * p[0].y) + abs(r[0].y * p[0].x)) + (abs(r[1].x * p[1].y) + abs(r[1].y * p[1].x))
        + (abs(r[2].x * p[2].y) + abs(r[2].y * p[2].x));
}

// The neutral band's half-width on `energy_drift`, `15·2⁻²³·(K + |V|)` (R-928).
fn dbg_drift_band_energy(r: array<vec2<f32>, 3>, p: array<vec2<f32>, 3>, m: vec3<f32>) -> f32 {
    return DBG_DRIFT_BAND_E * dbg_drift_rounding_energy(r, p, m);
}

// The neutral band's half-width on `Lz_drift`, `5·2⁻²³·S` (R-928).
fn dbg_drift_band_lz(r: array<vec2<f32>, 3>, p: array<vec2<f32>, 3>) -> f32 {
    return DBG_DRIFT_BAND_LZ * dbg_drift_rounding_lz(r, p);
}

// The neutral band, one definition for every place a current drift is read under `symlog`, the three drift views and
// a field ramp's `diverging` compaction (R-928, R-929): 0 where `|x| ≤ band`, the half-width `dbg_drift_band_energy`'s
// or `dbg_drift_band_lz`'s, since a difference of that size is the rounding of `H(r, p) − E_0` or `L_z(r, p) − Lz_0` in
// live f32 and not physics (R-652); else `x` as it is, so a drift past the band keeps its own place. A non-finite `x`
// or `band`, tested first on the bits, passes `x`, so the band never makes a non-finite operand neutral (`|x| ≤ ∞`
// holds for every `x`).
fn dbg_drift_banded(x: f32, band: f32) -> f32 {
    if (dbg_drift_non_finite(x) || dbg_drift_non_finite(band)) {
        return x;
    }
    return select(x, 0.0, abs(x) <= band);
}

// The value a drift view hands `dbg_drift`: under `symlog`, the band's (`dbg_drift_banded`); `lin` and `log` pass `x`
// through, having no band.
fn dbg_drift_shown(x: f32, compaction: u32, band: f32) -> f32 {
    if (compaction == DBG_DRIFT_LIN || compaction == DBG_DRIFT_LOG) {
        return x;
    }
    return dbg_drift_banded(x, band);
}

// A drift view's colour of the drift `x` (render contract Part 5; R-381; RQ-208, option 3): the absence NaN, by its
// exact bits, draws `debug_invalid(frag_xy)`, the hatch, under every compaction (R-136); any other value its place
// (`dbg_drift_place`) on its ramp: `symlog` and `lin`, the signed compactions, on `diverging`, or viridis under the
// palette swap (`viridis` true), where `symlog`'s pixel is `dbg_sentinel(s, frag_xy)`; `log`, unsigned, on viridis
// always, never on `diverging` (colour_composition §1.2).
fn dbg_drift(x: f32, compaction: u32, viridis: bool, lin_r: f32, eps: f32, frag_xy: vec2<f32>) -> vec3<f32> {
    if (is_absent_nan(x)) {
        return debug_invalid(frag_xy);
    }
    let t = dbg_drift_place(x, compaction, lin_r, eps);
    return dbg_drift_ramp(t, viridis || compaction == DBG_DRIFT_LOG);
}

// The drift suspect styling (render contract Part 5; R-379; REQ-TOOL-149), applied on a drift view's ramp output
// `rgb`, never on the hatch (`x` the absence NaN): lines one pixel wide every fourth pixel from the pixel
// `p = ⌊frag_xy⌋`, in framebuffer pixels, as the hatch's stripes are, drawn in ink, black where the ramp output's
// relative luminance (Rec. 709's weights, linear RGB) is at least 0.18, white below. `SUSPECT_ENERGY` (`energy`)
// drives the rows, where `p.y & 3` is 0; `SUSPECT_LZ` (`lz`) the columns, where `p.x & 3` is 0; both, a grid. Every
// other pixel, and every pixel of a sample neither predicate lights, is `rgb` unchanged.
fn dbg_suspect(rgb: vec3<f32>, x: f32, energy: bool, lz: bool, frag_xy: vec2<f32>) -> vec3<f32> {
    if (is_absent_nan(x)) {
        return rgb;
    }
    let p = vec2<i32>(floor(frag_xy));
    let line = (energy && (p.y & 3) == 0) || (lz && (p.x & 3) == 0);
    let ink = select(vec3<f32>(1.0), vec3<f32>(0.0), dot(rgb, vec3<f32>(0.2126, 0.7152, 0.0722)) >= 0.18);
    return select(rgb, ink, line);
}
