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
//! **Non-finite operands (R-436).** Each of the four operands, `K₀`, `V₀` and the fragment decode's `K` and `V` (whose
//! `−∞` at a zero pair separation the generated decode selects, as the decode stage does), is tested on its bits
//! before any arithmetic, `K₀ + V₀` included; a pixel with a non-finite one draws the invalid hatch and is no sample of
//! the comparison. The others enter the arithmetic as they are; a non-finite one is replaced by 0 by a select, never
//! summed.
//!
//! **The tolerance** (REQ-COL-060; R-297, R-133 as R-297 amends it). The fragment may compile with fast-math and the
//! kernel's decode stage doesn't (R-297), so the agreement compares within REQ-COL-060's calibrated tolerance, [`FACTOR`]
//! f32 `eps` applied to each operand: `|E − E₀| ≤ FACTOR·eps·(max(|K₀|, 1) + max(|V₀|, 1))`, the form TASK-M2-25 gave
//! it (applied per R-369 there: `E − E₀ = (K − K₀) + (V − V₀)`, each difference bounded by `FACTOR·eps·max(|x|, 1)`).
//! [`ratio`] is `|E − E₀|` over that bound: at most 1 is agreement. TASK-M2-29 measured the fragment-against-compute
//! differences on Metal and lavapipe (the `decode-agreement` gate's `survey` input) and proposed the factor; it is
//! provisional until the human confirms it at the M2 gate (R-71, R-182). REQ-DEC-043's factor stays the tolerance of
//! the fragment decode against the f64 `decodeOnly()`, not this one.
//!
//! **Its colour.** Agreement, ratio `ρ ≤ 1`, is green, `(0, 0.15 + 0.35ρ, 0)`, brighter as it nears the bound;
//! disagreement, `ρ > 1`, red, `(0.5 + 0.5·min(log₂ρ / 16, 1), 0, 0)`; a non-finite operand the hatch. [`colour`] is the
//! CPU twin.

use super::{Preset, SHARED_WGSL};
use crate::bind::ViewOutput;

/// The preset's name in the library.
pub const NAME: &str = "Agreement |E(fragment-decode) − E₀|";

/// REQ-COL-060's factor, in f32 `eps`: `fixtures/gates/decode-agreement/gate.json`'s threshold, proposed by TASK-M2-29
/// from its measurement and provisional until the human confirms it at the M2 gate (R-71, R-182).
pub const FACTOR: f32 = 256.0;

/// The factor's WGSL declaration, [`FACTOR`] written as a WGSL literal, so the shader compiles the one value.
fn factor_wgsl() -> String {
    format!(
        "
// REQ-COL-060's factor in f32 eps (render::presets::agreement::FACTOR; provisional, R-71).
const AGREEMENT_FACTOR: f32 = {FACTOR:?};"
    )
}

/// The comparison's WGSL after the factor: `agreement(k, v, k_0, v_0)` over the four operands' bits.
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

// The fragment decode's K and V against the payload's K₀ and V₀, each given as its bits, each tested on its bits
// before any arithmetic (R-436); a non-finite operand is replaced by 0 by a select, never summed.
fn agreement(k: u32, v: u32, k_0: u32, v_0: u32) -> Agreement {
    var a: Agreement;
    a.valid = !(preset_non_finite(k) || preset_non_finite(v) || preset_non_finite(k_0) || preset_non_finite(v_0));
    let ok = a.valid;
    let kf = select(0.0, bitcast<f32>(k), ok);
    let vf = select(0.0, bitcast<f32>(v), ok);
    let k0 = select(0.0, bitcast<f32>(k_0), ok);
    let v0 = select(0.0, bitcast<f32>(v_0), ok);
    a.e = kf + vf;
    a.e_0 = k0 + v0;
    let bound = AGREEMENT_FACTOR * AGREEMENT_EPS * (max(abs(k0), 1.0) + max(abs(v0), 1.0));
    a.ratio = abs(a.e - a.e_0) / bound;
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
    let d = frag_decode(agreement_point(l));
    return agreement(d.d.K_0, d.d.V_0, bitcast<u32>(rc.ic.K_0), bitcast<u32>(rc.ic.V_0));
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

/// The CPU twin of `agreement`: `None` where an operand is non-finite (by its bits), else `|E − E₀|` over the bound.
pub fn ratio(k: f32, v: f32, k_0: f32, v_0: f32) -> Option<f64> {
    if [k, v, k_0, v_0].iter().any(|x| super::non_finite_bits(*x)) {
        return None;
    }
    let (k, v, k_0, v_0) = (f64::from(k), f64::from(v), f64::from(k_0), f64::from(v_0));
    let bound =
        f64::from(FACTOR) * f64::from(f32::EPSILON) * (k_0.abs().max(1.0) + v_0.abs().max(1.0));
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
