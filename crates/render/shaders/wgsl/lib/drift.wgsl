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

// The neutral band's half-width as a fraction of the drift's reference: `k·2⁻²³` with `k = 128`, so 2⁻¹⁶. The bound
// behind `k` is the render contract's (Part 5, "The neutral band"). The CPU mirror is
// `crate::present::DRIFT_NEUTRAL_BAND`.
const DBG_DRIFT_NEUTRAL_BAND: f32 = 1.52587890625e-5;

// The value a drift view hands `dbg_drift`: under `symlog`, 0 where `|x| ≤ DBG_DRIFT_NEUTRAL_BAND·|reference|`, the
// reference `E_0` or `Lz_0`, since a difference of that size is the rounding of `H(r, p) − E_0` or `L_z(r, p) − Lz_0`
// in live f32 and not physics (R-652); else `x` as it is, so a drift past the band keeps its own place. `lin` and `log`
// pass `x` through, and so does the absence NaN, tested first by its bits. At a zero reference the band is empty.
fn dbg_drift_shown(x: f32, compaction: u32, reference: f32) -> f32 {
    if (is_absent_nan(x) || compaction == DBG_DRIFT_LIN || compaction == DBG_DRIFT_LOG) {
        return x;
    }
    return select(x, 0.0, abs(x) <= DBG_DRIFT_NEUTRAL_BAND * abs(reference));
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
