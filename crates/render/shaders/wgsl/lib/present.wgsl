// The presentation layer (render contract Part 5, "Presentation layer"): hand-written, small, reused by every debug
// view. It follows the shared prelude at assembly (`prelude.wgsl`), whose ramps, colour-space maps, absence test and
// `debug_invalid` it calls. Every helper returns linear RGB, the colour slot's space (render contract Part 2). The
// renderings are render contract Part 5's definitions (R-72; REQ-TOOL-122); the CPU mirror is
// `crates/render/src/present.rs`.

// The Okabe–Ito palette (Okabe & Ito 2002, "Color Universal Design"), 8-bit sRGB, in its published order: black,
// orange, sky blue, bluish green, yellow, blue, vermillion, reddish purple.
const DBG_OKABE_ITO: array<vec3<u32>, 8> = array<vec3<u32>, 8>(
    vec3<u32>(0u, 0u, 0u),
    vec3<u32>(230u, 159u, 0u),
    vec3<u32>(86u, 180u, 233u),
    vec3<u32>(0u, 158u, 115u),
    vec3<u32>(240u, 228u, 66u),
    vec3<u32>(0u, 114u, 178u),
    vec3<u32>(213u, 94u, 0u),
    vec3<u32>(204u, 121u, 167u),
);

// The golden ratio's fractional part, φ_g = (√5 − 1)/2, as a 32-bit fixed-point fraction: round(φ_g · 2³²). The
// golden-angle hue of class i is frac(i · φ_g) turns (dd_colouring §3.7), computed exactly as i · this, wrapping.
const DBG_GOLDEN_FIXED: u32 = 2654435769u;

// An 8-bit sRGB colour, decoded to linear RGB.
fn dbg_srgb8(c: vec3<u32>) -> vec3<f32> { return srgb_to_linear(vec3<f32>(c) / 255.0); }

// The neutral "not yet" grey of running samples (R-96; colour_composition §1.4), the colour a field view draws
// `d_min`'s unset value in (R-280): 8-bit sRGB #4E4E4E, OKLab L 0.424, as linear RGB, each channel the f32 nearest
// sRGB's decode of 78/255. Proposed, R-71 (REQ-COL-053, RQ-233): the human confirms it at the M1 gate. The CPU mirror
// is `crate::present::NOT_YET_SRGB8`.
const DBG_NOT_YET: vec3<f32> = vec3<f32>(0.07618538, 0.07618538, 0.07618538);

// Categorical: class `i` of `n`. With n ≤ 8, the Okabe–Ito palette, cycling (i mod 8); with n > 8, the golden angle
// for every class, the prelude's `hue_wheel` at frac(i · φ_g) turns, so adjacent classes sit ≈ 137.5° apart
// (dd_colouring §3.7). The wheel's OKLCH lightness and chroma (R-72) are the ledger's, in gamut at every hue.
fn dbg_cat(i: u32, n: u32) -> vec3<f32> {
    if (n <= 8u) {
        return dbg_srgb8(DBG_OKABE_ITO[i % 8u]);
    }
    return hue_wheel(f32(i * DBG_GOLDEN_FIXED) / 4294967296.0);
}

// Scalar: `x` on the fixed range [lo, hi], clamped, on the viridis ramp.
fn dbg_lin(x: f32, lo: f32, hi: f32) -> vec3<f32> {
    return ramp_viridis(range_norm(x, lo, hi, false, vec2<f32>(0.0, 0.0)));
}

// Scalar, log-compressed: s = ln(1 + |x|/eps), then t = 1 − 1/(1 + s) on the viridis ramp. Linear below `eps`
// (t ≈ |x|/eps), logarithmic above it; 0 maps to the ramp's start, |x| → ∞ to its end. `eps > 0` is the caller's,
// the magnitude where compression sets in.
fn dbg_log(x: f32, eps: f32) -> vec3<f32> {
    let s = log(1.0 + abs(x) / eps);
    return ramp_viridis(1.0 - 1.0 / (1.0 + s));
}

// Boolean: true green, false red: Okabe–Ito's bluish green #009E73 and vermillion #D55E00, a pair every common colour
// vision deficiency still tells apart.
fn dbg_flag(b: bool) -> vec3<f32> {
    return select(dbg_srgb8(DBG_OKABE_ITO[6]), dbg_srgb8(DBG_OKABE_ITO[3]), b);
}

// The PCG hash of `v` (Jarzynski and Olano 2020, "Hash Functions for GPU Rendering", JCGT 9(3), `pcg_hash`).
fn dbg_pcg(v: u32) -> u32 {
    let state = v * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

// A hash's low three bytes, low first, as 8-bit sRGB red, green, blue: the byte-to-RGB step of the hashed colours.
fn dbg_bytes_rgb(h: u32) -> vec3<f32> {
    return dbg_srgb8(vec3<u32>(h & 255u, (h >> 8u) & 255u, (h >> 16u) & 255u));
}

// A raw word as a hashed colour ("is it changing at all"): the PCG hash's low three bytes as 8-bit sRGB red, green,
// blue.
fn dbg_hash_u32(v: u32) -> vec3<f32> {
    return dbg_bytes_rgb(dbg_pcg(v));
}

// The whole-word hash of a `free_group_word`, a `vec4<u32>` (R-72; REQ-TOOL-155): `dbg_pcg` folded over the four
// limbs, `h = pcg(w ^ pcg(z ^ pcg(y ^ pcg(x))))`, then `dbg_hash_u32`'s byte-to-RGB step. Every bit of the word, its
// length included, reaches the colour.
fn dbg_hash_word(w: vec4<u32>) -> vec3<f32> {
    return dbg_bytes_rgb(dbg_pcg(w.w ^ dbg_pcg(w.z ^ dbg_pcg(w.y ^ dbg_pcg(w.x)))));
}

// Whether any of three floats is the absence NaN, by its exact bits (lowering Part 3a).
fn dbg_any_absent(v: vec3<f32>) -> bool {
    return is_absent_nan(v.x) || is_absent_nan(v.y) || is_absent_nan(v.z);
}

// The ICDescriptor masses as one ternary colour (R-72; REQ-TOOL-154): `(m0, m1, m2)` as linear RGB scaled by
// `1/max(mᵢ)`, so equal masses draw white and a vertex, one mass alone, its primary. Where no mass is positive, or one
// is the absence NaN, the colour is undefined and the hatch is drawn.
fn dbg_ternary(m: vec3<f32>, frag_xy: vec2<f32>) -> vec3<f32> {
    let top = max(m.x, max(m.y, m.z));
    if (dbg_any_absent(m) || !(top > 0.0)) {
        return debug_invalid(frag_xy);
    }
    return m / top;
}

// The direction cosines of a 3-vector as a colour (`n`, k = 3; render contract Part 5, live shape views): `½(v̂ + 1)`
// as linear RGB, `v̂ = v/‖v‖`. Where `‖v‖ > 0` fails, or a component is the absence NaN, the direction is undefined
// and the hatch is drawn.
fn dbg_dircos3(v: vec3<f32>, frag_xy: vec2<f32>) -> vec3<f32> {
    let norm = length(v);
    if (dbg_any_absent(v) || !(norm > 0.0)) {
        return debug_invalid(frag_xy);
    }
    return 0.5 * (v / norm + vec3<f32>(1.0));
}

// The direction cosines of a k = 6 vector as a colour (`r`, `p`, the shadow; R-72; REQ-TOOL-157): the six cosines
// `cᵢ = vᵢ/‖v‖` grouped by body, component `i` into channel `⌊3i/6⌋`, each channel the sum of its squared cosines,
// the body's share of `‖v‖²`; the three shares, which sum to 1, as linear RGB scaled by `1/max`, as the ternary masses
// are. The scale cancels `‖v‖²`, so the colour is `wᵢ/max(wᵢ)` with `wᵢ = ‖vᵢ‖²`, body `i`'s squared norm. Where
// `‖v‖ > 0` fails, or a component is the absence NaN, the direction is undefined and the hatch is drawn.
fn dbg_dircos6(v: array<vec2<f32>, 3>, frag_xy: vec2<f32>) -> vec3<f32> {
    let w = vec3<f32>(dot(v[0], v[0]), dot(v[1], v[1]), dot(v[2], v[2]));
    let absent = dbg_any_absent(vec3<f32>(v[0].x, v[1].x, v[2].x)) || dbg_any_absent(vec3<f32>(v[0].y, v[1].y, v[2].y));
    let top = max(w.x, max(w.y, w.z));
    if (absent || !(top > 0.0)) {
        return debug_invalid(frag_xy);
    }
    return w / top;
}

// A stored value's place on the viridis ramp with no range: 0.5 + 0.5 · x/(1 + |x|), so every finite value has its own
// place, 0 at the middle, −1 at a quarter, 1 at three quarters (R-79, R-136). `x` is clamped to ±1e30 first, so ±∞
// reach the ends.
fn dbg_literal(x: f32) -> f32 {
    let c = clamp(x, -1e30, 1e30);
    return 0.5 + 0.5 * c / (1.0 + abs(c));
}

// A value that may be absent: the absence NaN, by its exact bits, draws `debug_invalid(frag_xy)`, the hatch (R-136);
// any other value, a stored sentinel such as −1.0 included, shows as its literal value on the viridis ramp
// (`dbg_literal`; R-79).
fn dbg_sentinel(x: f32, frag_xy: vec2<f32>) -> vec3<f32> {
    if (is_absent_nan(x)) {
        return debug_invalid(frag_xy);
    }
    return ramp_viridis(dbg_literal(x));
}
