//! The DECODE preset (colour_composition §6; debug_tooling_plan §A; R-75; REQ-RENDER-027, REQ-TOOL-029): the
//! fragment-side decode of the pixel's chart point, coloured, before any physics. It certifies the decode rung: the
//! generated decode ([`super::DECODE_WGSL`]) runs at `ctx.chart.slice_uv`, places `ctx.chart.z` from it, and draws the
//! decoded system in three panels of each tile, by `ctx.tile.uv` (Y-up):
//!
//! | panel | where in the tile | colour |
//! |---|---|---|
//! | `z` components | the top third, `v ≥ 2/3`, split in thirds across `u` | `(z₀, z₁, z₂)`, `(z₃, z₄, z₅)`, `(z₆, z₇, 0)`, each through [`squash`] |
//! | masses, as ternary | the middle third, `1/3 ≤ v < 2/3` | `(m₀, m₁, m₂)` as RGB, the simplex's own point (`Σm = 1`) |
//! | body positions | the bottom third, `v < 1/3`, split in halves across `u` | `(x₀, x₁, x₂)` left, `(y₀, y₁, y₂)` right, each through [`squash`] |
//!
//! The `z` panel draws the invalid hatch (`debug_invalid`) on a chart whose `Φ` produces no latent (colour_composition
//! §3, REQ-COL-057), and every panel draws it where a value it shows is non-finite, tested on its bits before any
//! arithmetic (R-436; Discipline 2). A `DEGENERATE` decode keeps its finite outputs (dd_decoder §2), which the panels
//! show as they are, as debug fields show literal values (R-79).
//!
//! [`colour`] is the view's CPU twin, which the golden suite's CPU-decoded references are drawn with; [`probe`] is the
//! same decode written as raw words, which the tests read back to hold the fragment's `(m, r, p)` to the f64 decode.

use super::{Preset, SHARED_WGSL};
use crate::bind::ViewOutput;

/// The preset's name in the library.
pub const NAME: &str = "DECODE";

/// `squash(x) = ½ + ½·x/(1 + |x|)`: a signed value onto `(0, 1)`, `0` at `½`, odd about it, with no transcendental.
pub fn squash(x: f64) -> f64 {
    0.5 + 0.5 * x / (1.0 + x.abs())
}

/// The panels' WGSL, shared by the view and its words variant.
const PANELS: &str = "
// ½ + ½·x/(1 + |x|): a signed value onto (0, 1) (render::presets::decode::squash).
fn decode_squash(x: f32) -> f32 { return 0.5 + 0.5 * x / (1.0 + abs(x)); }

// A panel of three values: the invalid hatch where one is non-finite, tested on its bits before any arithmetic
// (R-436); otherwise the values through `decode_squash` where `squashed`, as they are where not.
fn decode_panel(v: vec3<f32>, squashed: vec3<bool>, frag_xy: vec2<f32>) -> vec3<f32> {
    let bad = preset_non_finite(bitcast<u32>(v.x)) || preset_non_finite(bitcast<u32>(v.y))
        || preset_non_finite(bitcast<u32>(v.z));
    let safe = select(v, vec3<f32>(0.0), vec3<bool>(bad));
    let s = vec3<f32>(decode_squash(safe.x), decode_squash(safe.y), decode_squash(safe.z));
    return select(select(safe, s, squashed), debug_invalid(frag_xy), vec3<bool>(bad));
}

// The DECODE preset's colour of the decode `d` at the within-tile coordinate `t` (Y-up) and pixel `frag_xy`, its `z`
// the chart lane's, placed from `d` (render::presets::decode).
fn decode_colour(d: FragDecode, z: array<f32, 8>, t: vec2<f32>, frag_xy: vec2<f32>) -> vec3<f32> {
    let all = vec3<bool>(true);
    if (t.y >= 2.0 / 3.0) {
        if (!d.z_latent) {
            return debug_invalid(frag_xy);
        }
        if (t.x < 1.0 / 3.0) {
            return decode_panel(vec3<f32>(z[0], z[1], z[2]), all, frag_xy);
        }
        if (t.x < 2.0 / 3.0) {
            return decode_panel(vec3<f32>(z[3], z[4], z[5]), all, frag_xy);
        }
        return decode_panel(vec3<f32>(z[6], z[7], 0.0), vec3<bool>(true, true, false), frag_xy);
    }
    if (t.y >= 1.0 / 3.0) {
        return decode_panel(d.m, vec3<bool>(false), frag_xy);
    }
    if (t.x < 0.5) {
        return decode_panel(vec3<f32>(d.r[0].x, d.r[1].x, d.r[2].x), all, frag_xy);
    }
    return decode_panel(vec3<f32>(d.r[0].y, d.r[1].y, d.r[2].y), all, frag_xy);
}
";

/// The view: the pixel's decode coloured by [`PANELS`]' `decode_colour`.
const VIEW: &str = "
fn view(rc: RenderContext, l0: Lanes) -> vec4<f32> {
    let d = frag_decode(l0.chart.slice_uv);
    var l = l0;
    l.chart.z = d.z;
    let frag_xy = vec2<f32>(l.screen.pixel) + vec2<f32>(0.5);
    return vec4<f32>(decode_colour(d, l.chart.z, l.tile.uv, frag_xy), 1.0);
}
";

/// The DECODE preset, locked.
pub fn preset() -> Preset {
    Preset {
        name: NAME,
        view: format!("{SHARED_WGSL}{PANELS}{VIEW}"),
        output: ViewOutput::Colour,
    }
}

/// The words a probe of the decode holds, by index: `(m, r, p)` 0–14, the tag 15, `z` 16–23, `z_latent` 24, the twelve
/// descriptor fields 25–36 (`ICDescriptor`'s order), and the chart point decoded, `slice_uv`, 37–38; 39 zero.
pub const PROBE_WORDS: usize = 40;

/// The probe's word groups, four words a group: one render each.
pub const PROBE_GROUPS: u32 = (PROBE_WORDS / 4) as u32;

/// The decode written as raw words, group `group` of [`PROBE_WORDS`] (words `4·group` to `4·group + 3`), one pixel per
/// sample: the DECODE preset's decode at the pixel's chart point, read back by the tests (REQ-TOOL-029).
pub fn probe(group: u32) -> Preset {
    let view = format!(
        "{SHARED_WGSL}{PANELS}
// The DECODE preset's decode as words (render::presets::decode::PROBE_WORDS), group {group}.
fn view(rc: RenderContext, l0: Lanes) -> vec4<u32> {{
    let d = frag_decode(l0.chart.slice_uv);
    var l = l0;
    l.chart.z = d.z;
    var w: array<u32, {PROBE_WORDS}>;
    w[0] = bitcast<u32>(d.m.x);
    w[1] = bitcast<u32>(d.m.y);
    w[2] = bitcast<u32>(d.m.z);
    for (var b = 0u; b < 3u; b++) {{
        w[3u + 2u * b] = bitcast<u32>(d.r[b].x);
        w[4u + 2u * b] = bitcast<u32>(d.r[b].y);
        w[9u + 2u * b] = bitcast<u32>(d.p[b].x);
        w[10u + 2u * b] = bitcast<u32>(d.p[b].y);
    }}
    w[15] = d.tag;
    for (var k = 0u; k < 8u; k++) {{
        w[16u + k] = bitcast<u32>(l.chart.z[k]);
    }}
    w[24] = select(0u, 1u, d.z_latent);
    w[25] = d.d.m0;
    w[26] = d.d.m1;
    w[27] = d.d.m2;
    w[28] = d.d.q_mass;
    w[29] = d.d.rho_mag;
    w[30] = d.d.lambda_mag;
    w[31] = d.d.rho_ratio;
    w[32] = d.d.rho_angle;
    w[33] = d.d.K_0;
    w[34] = d.d.V_0;
    w[35] = d.d.virial_ratio;
    w[36] = d.d.r_min_pair_0;
    w[37] = bitcast<u32>(l0.chart.slice_uv.x);
    w[38] = bitcast<u32>(l0.chart.slice_uv.y);
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
        view,
        output: ViewOutput::Words,
    }
}

/// One decode, at any precision, as the view reads it: `Φ`'s `z` and whether it is a latent, the masses and the
/// positions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shown {
    pub z: [f64; 8],
    pub z_latent: bool,
    pub m: [f64; 3],
    pub r: [[f64; 2]; 3],
}

/// The panel of three values `v` at the pixel `frag_xy`, `squashed` per value: the WGSL `decode_panel` on the CPU.
fn panel(v: [f64; 3], squashed: [bool; 3], frag_xy: [f64; 2]) -> [f64; 3] {
    if v.iter().any(|x| !x.is_finite()) {
        return crate::present::debug_invalid(frag_xy);
    }
    [0, 1, 2].map(|k| if squashed[k] { squash(v[k]) } else { v[k] })
}

/// The DECODE preset's colour of `d` at the within-tile coordinate `t` (Y-up) and pixel `frag_xy`, linear RGB: the
/// view's `decode_colour` on the CPU.
pub fn colour(d: &Shown, t: [f64; 2], frag_xy: [f64; 2]) -> [f64; 3] {
    let all = [true; 3];
    let z = &d.z;
    if t[1] >= 2.0 / 3.0 {
        if !d.z_latent {
            return crate::present::debug_invalid(frag_xy);
        }
        return if t[0] < 1.0 / 3.0 {
            panel([z[0], z[1], z[2]], all, frag_xy)
        } else if t[0] < 2.0 / 3.0 {
            panel([z[3], z[4], z[5]], all, frag_xy)
        } else {
            panel([z[6], z[7], 0.0], [true, true, false], frag_xy)
        };
    }
    if t[1] >= 1.0 / 3.0 {
        return panel(d.m, [false; 3], frag_xy);
    }
    let r = &d.r;
    if t[0] < 0.5 {
        panel([r[0][0], r[1][0], r[2][0]], all, frag_xy)
    } else {
        panel([r[0][1], r[1][1], r[2][1]], all, frag_xy)
    }
}
