//! The agreement preset `|E(fragment-decode) − E₀|` (colour_composition §6, "Agreement presets"; R-116, R-133, R-297,
//! R-436; REQ-COL-006): the fragment's decode of the sample against the kernel decode stage's own, read from the
//! payload, a live check of the translation (two compilation paths of one source) and of write-addressing (a dispatch
//! scramble shows as spatial disagreement).
//!
//! **What it compares.** `E₀ = K₀ + V₀`, the decode stage's `ICDescriptor` fields as the payload stores them
//! (`ctx.ic`; R-86, R-133: not `SimState.E_0`), against `E(fragment-decode) = K + V`, the generated decode's own
//! descriptor at the sample's chart point: its quad's centre and half-width at the sample's quad-local coordinate, the
//! tile's centre, `ctx.quad.uv` less the pixel's offset in its tile, where the decode stage decoded it.
//!
//! **Non-finite operands (R-436).** Each operand, `K₀`, `V₀` and the fragment decode's `K` and `V` (whose `−∞` at a
//! zero pair separation the generated decode selects, as the decode stage does), and the fragment decode's `m` and `r`
//! the scale reads, is tested on its bits before any arithmetic, `K₀ + V₀` included; a pixel with a non-finite one
//! draws the invalid hatch and is no sample of the comparison. The others enter the arithmetic as they are; a
//! non-finite one is replaced by 0 by a select, never summed.
//!
//! **The tolerance** (REQ-COL-060; R-297, R-133 as R-297 amends it; colour_composition §6, "The tolerance's scale",
//! R-72). The fragment may compile with fast-math and the kernel's decode stage doesn't (R-297), so the agreement
//! compares within REQ-COL-060's calibrated tolerance, [`FACTOR`] f32 `eps` of the conditioning-aware scale:
//! `|E − E₀| ≤ FACTOR·eps·(max(|K₀|, 1) + max(|V₀|, 1) + C)`, `C` = [`conditioning`] of the fragment decode's own
//! `(m, r)`, `Σ_{i<j} mᵢmⱼ·wᵢⱼ/ŝᵢⱼ²` with `wᵢⱼ = ‖rᵢ‖∞ + ‖rⱼ‖∞` and `ŝᵢⱼ = max(‖rᵢ − rⱼ‖∞, eps·max(wᵢⱼ, δ))`,
//! `δ` = [`FLOOR`] `= 2⁻³²`: the conditioning of `V` at f32, which beside a binary collision grows as `wᵢⱼ/rᵢⱼ` up to
//! `1/eps` (RQ-623, option (b), decided by the orchestrator per R-369). The floor is the separation's own rounding,
//! `eps·wᵢⱼ`, at every size of `wᵢⱼ`; `δ` only keeps it positive where both positions are 0, and no decoded system
//! reaches it (`wᵢⱼ ≥ √m_k/2 ≥ 2⁻²²` under `I = 1` at the masses' floors). Each pair's term is at most
//! `(1/(4·eps·δ))·(1/eps) = 2⁷⁶` for masses on `[0, 1]` with `M = 1`, so `C` is finite at every pixel, the collision
//! pole included, with no division by zero (`ŝᵢⱼ ≥ eps·δ = 2⁻⁵⁵`) and, for positions below `2⁶²`, no intermediate
//! overflowing under any association a fast-math compiler picks (it may form `mᵢmⱼ·wᵢⱼ/ŝᵢⱼ²`; R-436). The positions
//! are below `2⁶²` for every mass a decode reaches: `‖rᵢ‖ ≤ 1/√mᵢ` under `I = 1`, and the mass links floor every mass
//! far above `2⁻¹²⁴` (dd_decoder §3.1; `stick_breaking`'s `ε_μ = 10⁻⁶` gives at least `ε_μ²/4`, `softmax_tanh` at
//! `μ_max` up to 10 about `2×10⁻⁹`, the ternary plot `ε_m = 10⁻⁴`), so `‖rᵢ‖ < 2²¹`. The positions are finite where the
//! masses are finite and nonzero; a mass rounded to 0 would give a non-finite `r`, and `m` and `r` are tested on their
//! bits with the other operands, so a non-finite one draws the hatch (colour_composition §6). [`ratio`] is `|E − E₀|`
//! over that bound: at most 1 is agreement. TASK-M2-29 measured the fragment-against-compute differences on Metal and
//! lavapipe (the `decode-agreement` gate's `survey` and `collision` inputs) and proposed the factor; it is provisional
//! until the human confirms it at the M2 gate (R-71, R-182). REQ-DEC-043's factor stays the tolerance of the fragment decode
//! against the f64 `decodeOnly()`, not this one.
//!
//! **Its colour.** Agreement, ratio `ρ ≤ 1`, is green, `(0, 0.15 + 0.35ρ, 0)`, brighter as it nears the bound;
//! disagreement, `ρ > 1`, red, `(0.5 + 0.5·min(log₂ρ / 16, 1), 0, 0)`; a non-finite operand the hatch. [`colour`] is the
//! CPU twin.

use super::{Preset, SHARED_WGSL};
use crate::bind::ViewOutput;

/// The preset's name in the library.
pub const NAME: &str = "Agreement |E(fragment-decode) − E₀|";

/// REQ-COL-060's factor, in f32 `eps` of the conditioning-aware scale: `fixtures/gates/decode-agreement/gate.json`'s
/// threshold, proposed by TASK-M2-29 from its measurement and provisional until the human confirms it at the M2 gate
/// (R-71, R-182): the smallest power of two at least three times the worst factor measured on the gate's lattices,
/// up to 512² per chart, and its collision windows, on Metal and lavapipe (19.5 on Metal, `latent` at 512²).
pub const FACTOR: f32 = 64.0;

/// The factor's and the floor's WGSL declarations, [`FACTOR`] and [`FLOOR`] written as WGSL literals, so the shader
/// compiles the one value of each.
fn factor_wgsl() -> String {
    format!(
        "
// REQ-COL-060's factor in f32 eps (render::presets::agreement::FACTOR; provisional, R-71).
const AGREEMENT_FACTOR: f32 = {FACTOR:?};

// The separation floor's delta, 2^-32 (render::presets::agreement::FLOOR).
const AGREEMENT_FLOOR: f32 = {FLOOR:?};"
    )
}

/// The separation floor's `δ`, `2⁻³²` (colour_composition §6): `ŝᵢⱼ ≥ eps·max(wᵢⱼ, δ)`, positive where both positions
/// are 0 and below every `wᵢⱼ` a decoded system reaches.
pub const FLOOR: f32 = 1.0 / 4_294_967_296.0;

/// The comparison's WGSL after the factor: `agreement_of(d, k_0, v_0)`, the fragment decode `d` against the payload's
/// `K₀` and `V₀` within colour_composition §6's conditioning-aware scale of `d`'s own `(m, r)`, which the view draws.
const COMPARE: &str = "
// f32's eps, 2^-23.
const AGREEMENT_EPS: f32 = 1.1920929e-7;

// One comparison: `ratio`, |E − E₀| over the tolerance; `valid`, every operand finite; `e` and `e_0` the two energies.
struct Agreement {
    ratio: f32,
    valid: bool,
    e: f32,
    e_0: f32,
}

// The max-norm of `v`: the larger magnitude of its two components.
fn agreement_norm(v: vec2<f32>) -> f32 {
    return max(abs(v.x), abs(v.y));
}

// One pair's conditioning term, (mi·mj/s)·(w/s), w = |ri|∞ + |rj|∞, s = max(|ri − rj|∞, eps·max(w, delta)): each
// factor bounded, by 1/(4·eps·delta) and 1/eps, s at least eps·delta, so the term is finite in any association
// (colour_composition §6).
fn agreement_pair(mi: f32, mj: f32, ri: vec2<f32>, rj: vec2<f32>) -> f32 {
    let w = agreement_norm(ri) + agreement_norm(rj);
    let s = max(agreement_norm(ri - rj), AGREEMENT_EPS * max(w, AGREEMENT_FLOOR));
    return (mi * mj / s) * (w / s);
}

// The scale's conditioning term of `(m, r)`, its three pairs' sum.
fn agreement_conditioning(m: vec3<f32>, r: array<vec2<f32>, 3>) -> f32 {
    return agreement_pair(m.x, m.y, r[0], r[1]) + agreement_pair(m.x, m.z, r[0], r[2])
        + agreement_pair(m.y, m.z, r[1], r[2]);
}

// Whether `m` and `r` are finite, each component tested on its bits (R-436).
fn agreement_finite_mr(m: vec3<f32>, r: array<vec2<f32>, 3>) -> bool {
    let mb = bitcast<vec3<u32>>(m);
    let r0 = bitcast<vec2<u32>>(r[0]);
    let r1 = bitcast<vec2<u32>>(r[1]);
    let r2 = bitcast<vec2<u32>>(r[2]);
    return !(preset_non_finite(mb.x) || preset_non_finite(mb.y) || preset_non_finite(mb.z)
        || preset_non_finite(r0.x) || preset_non_finite(r0.y) || preset_non_finite(r1.x) || preset_non_finite(r1.y)
        || preset_non_finite(r2.x) || preset_non_finite(r2.y));
}

// The fragment decode's K and V against the payload's K₀ and V₀, each given as its bits, each tested on its bits
// before any arithmetic (R-436); a non-finite operand is replaced by 0 by a select, never summed. `c` is the scale's
// conditioning term, finite and at least 0.
fn agreement_scaled(k: u32, v: u32, k_0: u32, v_0: u32, c: f32) -> Agreement {
    var a: Agreement;
    a.valid = !(preset_non_finite(k) || preset_non_finite(v) || preset_non_finite(k_0) || preset_non_finite(v_0));
    let ok = a.valid;
    let kf = select(0.0, bitcast<f32>(k), ok);
    let vf = select(0.0, bitcast<f32>(v), ok);
    let k0 = select(0.0, bitcast<f32>(k_0), ok);
    let v0 = select(0.0, bitcast<f32>(v_0), ok);
    a.e = kf + vf;
    a.e_0 = k0 + v0;
    let bound = AGREEMENT_FACTOR * AGREEMENT_EPS * (max(abs(k0), 1.0) + max(abs(v0), 1.0) + c);
    a.ratio = abs(a.e - a.e_0) / bound;
    return a;
}

// The comparison of the four operands with no conditioning term, the scale's per-operand terms alone: the per-operand
// entry, kept for TASK-M2-25's tests of the validity and colour path; the view does not call it (it calls agreement_of).
fn agreement(k: u32, v: u32, k_0: u32, v_0: u32) -> Agreement {
    return agreement_scaled(k, v, k_0, v_0, 0.0);
}

// The fragment decode `d` against the payload's K₀ and V₀ (bits), within the conditioning-aware scale of d's own
// (m, r); a non-finite m or r makes the sample invalid, and is replaced by 0 by a select before the scale reads it.
fn agreement_of(d: FragDecode, k_0: u32, v_0: u32) -> Agreement {
    let ok = agreement_finite_mr(d.m, d.r);
    let m = select(vec3<f32>(0.0), d.m, ok);
    var r: array<vec2<f32>, 3>;
    r[0] = select(vec2<f32>(0.0), d.r[0], ok);
    r[1] = select(vec2<f32>(0.0), d.r[1], ok);
    r[2] = select(vec2<f32>(0.0), d.r[2], ok);
    var a = agreement_scaled(d.d.K_0, d.d.V_0, k_0, v_0, agreement_conditioning(m, r));
    a.valid = a.valid && ok;
    return a;
}

// The agreement's colour: green within the bound, red past it, the hatch where an operand is non-finite.
fn agreement_colour(a: Agreement, frag_xy: vec2<f32>) -> vec3<f32> {
    if (!a.valid) {
        return debug_invalid(frag_xy);
    }
    if (a.ratio <= 1.0) {
        return vec3<f32>(0.0, 0.15 + 0.35 * a.ratio, 0.0);
    }
    return vec3<f32>(0.5 + 0.5 * min(log2(a.ratio) / 16.0, 1.0), 0.0, 0.0);
}

// The sample's own chart point: its quad's frame at the tile's centre, ctx.quad.uv less the pixel's offset in its tile.
fn agreement_point(l: Lanes) -> vec2<f32> {
    let t = l.quad.uv + (vec2<f32>(0.5) - l.tile.uv) / f32(l.tile.N);
    return l.quad.centre + l.quad.half_width * (2.0 * t - vec2<f32>(1.0));
}

// The fragment decode of the sample against its payload descriptor (`ctx.ic`).
fn agreement_at(rc: RenderContext, l: Lanes) -> Agreement {
    return agreement_of(frag_decode(agreement_point(l)), bitcast<u32>(rc.ic.K_0), bitcast<u32>(rc.ic.V_0));
}
";

/// The view.
const VIEW: &str = "
fn view(rc: RenderContext, l: Lanes) -> vec4<f32> {
    let frag_xy = vec2<f32>(l.screen.pixel) + vec2<f32>(0.5);
    return vec4<f32>(agreement_colour(agreement_at(rc, l), frag_xy), 1.0);
}
";

/// The words view: `(ratio's bits, valid, E's bits, E₀'s bits)`, which the decode-agreement gate reads.
const WORDS: &str = "
fn view(rc: RenderContext, l: Lanes) -> vec4<u32> {
    let a = agreement_at(rc, l);
    return vec4<u32>(bitcast<u32>(a.ratio), select(0u, 1u, a.valid), bitcast<u32>(a.e), bitcast<u32>(a.e_0));
}
";

/// The agreement preset, locked.
pub fn preset() -> Preset {
    Preset {
        name: NAME,
        view: format!("{SHARED_WGSL}{}{COMPARE}{VIEW}", factor_wgsl()),
        output: ViewOutput::Colour,
    }
}

/// The agreement preset's comparison as words, `(ratio, valid, E, E₀)` per pixel, for the gate and the tests.
pub fn words() -> Preset {
    Preset {
        name: NAME,
        view: format!("{SHARED_WGSL}{}{COMPARE}{WORDS}", factor_wgsl()),
        output: ViewOutput::Words,
    }
}

/// The words of [`operands`]: the fragment decode's `K`, `V`, `m` and `r`, and a zero word.
pub const OPERAND_WORDS: usize = 12;

/// The operand view's word groups, four words a group: one render each.
pub const OPERAND_GROUPS: u32 = (OPERAND_WORDS / 4) as u32;

/// The operands the preset tests at each pixel, as raw words, group `group` of [`OPERAND_WORDS`] (words `4·group` to
/// `4·group + 3`), one pixel per sample: the fragment decode's `K`, `V`, `m` (three) and `r` (six, body by body, `x`
/// then `y`), then 0, as their bits, of the decode the preset draws, `frag_decode(agreement_point(l))`, compiled after
/// the preset's own WGSL. The `decode-agreement` gate tests them on their bits for the hatch's placement (R-436).
pub fn operands(group: u32) -> Preset {
    let view = format!(
        "
// The agreement preset's fragment decode as words (render::presets::agreement::OPERAND_WORDS), group {group}.
fn view(rc: RenderContext, l: Lanes) -> vec4<u32> {{
    let d = frag_decode(agreement_point(l));
    let mb = bitcast<vec3<u32>>(d.m);
    var w: array<u32, {OPERAND_WORDS}>;
    w[0] = d.d.K_0;
    w[1] = d.d.V_0;
    w[2] = mb.x;
    w[3] = mb.y;
    w[4] = mb.z;
    for (var b = 0u; b < 3u; b++) {{
        let rb = bitcast<vec2<u32>>(d.r[b]);
        w[5u + 2u * b] = rb.x;
        w[6u + 2u * b] = rb.y;
    }}
    w[11] = 0u;
    return vec4<u32>(w[{a}], w[{b}], w[{c}], w[{e}]);
}}
",
        a = 4 * group,
        b = 4 * group + 1,
        c = 4 * group + 2,
        e = 4 * group + 3,
    );
    Preset {
        name: NAME,
        view: format!("{SHARED_WGSL}{}{COMPARE}{view}", factor_wgsl()),
        output: ViewOutput::Words,
    }
}

/// The max-norm of `v`, at f64.
fn norm(v: [f64; 2]) -> f64 {
    v[0].abs().max(v[1].abs())
}

/// The CPU twin of `agreement_conditioning`, at f64 from the f32 `m` and `r`: `Σ_{i<j} (mᵢmⱼ/ŝᵢⱼ)·(wᵢⱼ/ŝᵢⱼ)`, with
/// `wᵢⱼ = ‖rᵢ‖∞ + ‖rⱼ‖∞` and `ŝᵢⱼ = max(‖rᵢ − rⱼ‖∞, eps·max(wᵢⱼ, δ))`, `δ` = [`FLOOR`] (colour_composition §6).
pub fn conditioning(m: [f32; 3], r: [[f32; 2]; 3]) -> f64 {
    let m = m.map(f64::from);
    let r = r.map(|p| p.map(f64::from));
    let eps = f64::from(f32::EPSILON);
    let pair = |i: usize, j: usize| {
        let w = norm(r[i]) + norm(r[j]);
        let s = norm([r[i][0] - r[j][0], r[i][1] - r[j][1]]).max(eps * w.max(f64::from(FLOOR)));
        (m[i] * m[j] / s) * (w / s)
    };
    pair(0, 1) + pair(0, 2) + pair(1, 2)
}

/// The CPU twin of `agreement_of`: `None` where an operand is non-finite (by its bits), `K`, `V`, `K₀`, `V₀` or the
/// fragment decode's `m` and `r`, else `|E − E₀|` over the bound `FACTOR·eps·(max(|K₀|, 1) + max(|V₀|, 1) + C)`, `C`
/// the [`conditioning`] of `(m, r)`.
pub fn ratio(k: f32, v: f32, k_0: f32, v_0: f32, m: [f32; 3], r: [[f32; 2]; 3]) -> Option<f64> {
    let mut operands = [k, v, k_0, v_0]
        .into_iter()
        .chain(m)
        .chain(r.into_iter().flatten());
    if operands.any(super::non_finite_bits) {
        return None;
    }
    let (k, v, k_0, v_0) = (f64::from(k), f64::from(v), f64::from(k_0), f64::from(v_0));
    let scale = k_0.abs().max(1.0) + v_0.abs().max(1.0) + conditioning(m, r);
    let bound = f64::from(FACTOR) * f64::from(f32::EPSILON) * scale;
    Some(((k + v) - (k_0 + v_0)).abs() / bound)
}

/// The CPU twin of `agreement_colour`, linear RGB: the hatch for `None`.
pub fn colour(ratio: Option<f64>, frag_xy: [f64; 2]) -> [f64; 3] {
    match ratio {
        None => crate::present::debug_invalid(frag_xy),
        Some(r) if r <= 1.0 => [0.0, 0.15 + 0.35 * r, 0.0],
        Some(r) => [0.5 + 0.5 * (r.log2() / 16.0).min(1.0), 0.0, 0.0],
    }
}
