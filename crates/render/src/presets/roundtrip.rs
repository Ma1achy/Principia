//! The ROUNDTRIP preset (inverse-encode contract Part 8, "New debug view: `ROUNDTRIP`"; colour_composition §6;
//! debug_tooling_plan §A; render_contract Part 6, "Cross-check views"; R-75, R-116, R-297, R-437; REQ-RENDER-025,
//! REQ-ENC-002): per pixel, `z → D → E → D` and the physical residual `‖D(z) − D(E(D(z)))‖_phys`, recomputed wholly in
//! the fragment and log-scaled, the encode-side sibling of the invariant-chart gradient. It is a fragment preset, never
//! a kernel mode (R-75): the kernel's variants stay the bring-up mode alone.
//!
//! **The trip** (R-116). The first `D` is the chart's generated decode at the pixel's `ctx.chart.slice_uv`
//! ([`super::DECODE_WGSL`]'s chart section, `frag_decode`), which places `ctx.chart.z`; `E` is the generated encode
//! ([`super::ENCODE_WGSL`], `frag_encode`), whose `atan2`s take their defined pole values by a select (R-437); the second
//! `D` is the generated free decode of the encode's `z` (the `free` section, `frag_decode_free`). All three are the
//! translation of the one Rust source (`kernel::stage::fragment`), none written by hand.
//!
//! **The residual** is inverse-encode Part 4's `‖·‖_phys` (REQ-ENC-029), as `validation::roundtrip::phys` measures it:
//! over the three bodies, the largest of `|Δmᵢ|`, `‖Δrᵢ‖/R_x` and `‖Δpᵢ‖/P_x`, `R_x` and `P_x` the first decode's extents
//! in positions and momenta, floored at 1. Its tolerance is `ε_phys = k·ε_R` (REQ-ENC-024), `k` the register's
//! `eps_phys` and `ε_R` f32's epsilon: R-297's stated tolerance for ROUNDTRIP, with the fragment compiled as the backend
//! compiles it. [`ratio`] is the residual over `ε_phys`: at most 1 is the round trip within tolerance.
//!
//! **Tagged expected** (Part 8: "Legitimate bright regions: clamp boundaries, feasibility edges, the mirror tie —
//! tagged expected, not bugs"), each read off what the trip itself reports, by [`Class`], in this order:
//! - **feasibility**: either decode is `DEGENERATE` (its tag word non-zero; the invariant charts' infeasible pairs, an
//!   energy target out of reach), so the state is not the pixel's;
//! - **clamp**: the encode reports `lookup_clamped`, a control past its clamp (notices bits 8 to 15) or the coincident
//!   inner pair (bit 0; R-13);
//! - **mirror tie**: the encode reports `lookup_mirrored` (bit 1). A decoded state is on `C`'s section, `λ̃_y ≥ 0`, so
//!   on it the mirror fires only where rounding puts `λ̃_y` past `−δ_λ`, at the tie (R-82).
//!
//! **Its colour** (Discipline 1: ROUNDTRIP-red means the same thing every time). The brightness is the residual
//! log-scaled from `ε_phys`: `b = min(log₂ ρ / SPAN, 1)` for `ρ > 1`, 0 at or below it, [`SPAN`] octaves to full scale.
//! An untagged pixel is `(b, 0, 0)`: black within tolerance, red past it, the one colour with a red channel, so any red
//! is a defect. A tagged pixel draws its tag's hue with no red, at `FLOOR + (1 − FLOOR)·b`, so its tag shows even
//! where its residual stays small: feasibility cyan `(0, t, t)`, clamp blue `(0, 0, t)`, mirror tie green `(0, t, 0)`.
//! A non-finite value among the trip's states, or a non-finite residual, draws the invalid hatch (Discipline 2), tested
//! on its bits before any arithmetic (R-436). [`colour`] is the CPU twin.

use super::{section, Preset, DECODE_WGSL, ENCODE_WGSL, SHARED_WGSL};
use crate::bind::ViewOutput;

/// The preset's name in the library.
pub const NAME: &str = "ROUNDTRIP";

/// The octaves of `ρ = residual/ε_phys` the brightness spans, from 0 at `ε_phys` to full at `2^SPAN·ε_phys`: 24, so at
/// f32 (`ε_phys ≈ 7.6·10⁻⁶`) full scale is a residual of about 128, past any state's order-1 extent.
pub const SPAN: f64 = 24.0;

/// A tagged pixel's least brightness, so its tag shows where its residual is within tolerance.
pub const FLOOR: f64 = 0.25;

/// `ε_phys` at f32, `k·2⁻²³`, `k` the register's `eps_phys` (REQ-ENC-024; provisional until the M2 gate, R-71).
pub fn eps_phys() -> f64 {
    ledger::constants::EPS_PHYS.number() * f64::from(f32::EPSILON)
}

/// What a pixel's trip is, by the first that holds, in this order: [`Class::Invalid`] a non-finite value, then the
/// three tags, then [`Class::Untagged`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    /// Every value finite, no tag: held to `ε_phys`.
    Untagged = 0,
    /// Either decode `DEGENERATE`.
    Feasibility = 1,
    /// The encode's `lookup_clamped`.
    Clamp = 2,
    /// The encode's `lookup_mirrored`.
    MirrorTie = 3,
    /// A non-finite value among the trip's states or its residual.
    Invalid = 4,
}

impl Class {
    /// The class of a trip whose decodes' tag words are `tag` and `tag_free` and whose encode's notices word is
    /// `notices`, `finite` where every value is finite.
    pub fn of(finite: bool, tag: u32, tag_free: u32, notices: u32) -> Class {
        if !finite {
            Class::Invalid
        } else if tag != 0 || tag_free != 0 {
            Class::Feasibility
        } else if notices & CLAMPED != 0 {
            Class::Clamp
        } else if notices & MIRRORED != 0 {
            Class::MirrorTie
        } else {
            Class::Untagged
        }
    }

    /// The class of the words view's code, `None` for a code it never writes.
    pub fn from_code(code: u32) -> Option<Class> {
        [
            Class::Untagged,
            Class::Feasibility,
            Class::Clamp,
            Class::MirrorTie,
            Class::Invalid,
        ]
        .into_iter()
        .find(|c| *c as u32 == code)
    }
}

/// The notices bits that are `lookup_clamped`: the coincident inner pair (bit 0) and the eight controls (bits 8 to 15).
pub const CLAMPED: u32 = 0xff01;

/// The notices bit that is `lookup_mirrored`.
pub const MIRRORED: u32 = 2;

/// `‖y − x‖_phys` (REQ-ENC-029), `x` the reference: the largest of `|Δmᵢ|`, `‖Δrᵢ‖/R_x` and `‖Δpᵢ‖/P_x`, `R_x` and
/// `P_x` `x`'s extents floored at 1. The CPU twin of the view's `roundtrip_phys`, each state `(m, r, p)`.
pub fn residual(x: &State, y: &State) -> f64 {
    let norm = |v: [f64; 2]| v[0].hypot(v[1]);
    let extent = |v: &[[f64; 2]; 3]| v.iter().map(|w| norm(*w)).fold(1.0f64, f64::max);
    let (r_x, p_x) = (extent(&x.r), extent(&x.p));
    (0..3)
        .flat_map(|i| {
            [
                (y.m[i] - x.m[i]).abs(),
                norm([y.r[i][0] - x.r[i][0], y.r[i][1] - x.r[i][1]]) / r_x,
                norm([y.p[i][0] - x.p[i][0], y.p[i][1] - x.p[i][1]]) / p_x,
            ]
        })
        .fold(0.0f64, f64::max)
}

/// The words view's tag word: the first decode's tag word `tag`, and the free decode's `tag_free` shifted by 8. Each
/// tag word is below 256 (`kernel::stage::decode::tag`'s 0 to 5), so the two never share a bit.
pub fn tags(tag: u32, tag_free: u32) -> u32 {
    tag | (tag_free << 8)
}

/// A state `(m, r, p)` as the twin reads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    pub m: [f64; 3],
    pub r: [[f64; 2]; 3],
    pub p: [[f64; 2]; 3],
}

impl State {
    /// Whether every value is finite.
    pub fn finite(&self) -> bool {
        self.m
            .iter()
            .chain(self.r.as_flattened())
            .chain(self.p.as_flattened())
            .all(|x| x.is_finite())
    }
}

/// `residual / ε_phys`.
pub fn ratio(residual: f64) -> f64 {
    residual / eps_phys()
}

/// The brightness of `ρ`: 0 at or below 1, `log₂ ρ / SPAN` above it, at most 1.
pub fn brightness(ratio: f64) -> f64 {
    (ratio.max(1.0).log2() / SPAN).min(1.0)
}

/// The view's colour of a trip of class `class` and ratio `ratio` at the pixel `frag_xy`, linear RGB: the CPU twin of
/// `roundtrip_colour`.
pub fn colour(class: Class, ratio: f64, frag_xy: [f64; 2]) -> [f64; 3] {
    let b = brightness(ratio);
    let t = FLOOR + (1.0 - FLOOR) * b;
    match class {
        Class::Invalid => crate::present::debug_invalid(frag_xy),
        Class::Untagged => [b, 0.0, 0.0],
        Class::Feasibility => [0.0, t, t],
        Class::Clamp => [0.0, 0.0, t],
        Class::MirrorTie => [0.0, t, 0.0],
    }
}

/// The trip's WGSL: `roundtrip_of(d)`, from the chart's decode `d`, the encode and the free decode, the residual and
/// the class; and `roundtrip_colour`.
fn trip() -> String {
    format!(
        "
// ε_phys at f32, k·2^-23, k the register's eps_phys (render::presets::roundtrip::eps_phys; REQ-ENC-024), the span and
// floor of the brightness (render::presets::roundtrip::SPAN, FLOOR).
const ROUNDTRIP_EPS_PHYS: f32 = {eps:e};
const ROUNDTRIP_SPAN: f32 = {SPAN:?};
const ROUNDTRIP_FLOOR: f32 = {FLOOR:?};

// One trip: the residual over ε_phys, the class (render::presets::roundtrip::Class), and what it read.
struct RoundTrip {{
    ratio: f32,
    kind: u32,
    notices: u32,
    tags: u32,
}}

// Whether any of the state's fifteen values is non-finite, tested on its bits (R-436).
fn roundtrip_bad(m: vec3<f32>, r: array<vec2<f32>, 3>, p: array<vec2<f32>, 3>) -> bool {{
    var bad = preset_non_finite(bitcast<u32>(m.x)) || preset_non_finite(bitcast<u32>(m.y))
        || preset_non_finite(bitcast<u32>(m.z));
    for (var i = 0u; i < 3u; i++) {{
        bad = bad || preset_non_finite(bitcast<u32>(r[i].x)) || preset_non_finite(bitcast<u32>(r[i].y))
            || preset_non_finite(bitcast<u32>(p[i].x)) || preset_non_finite(bitcast<u32>(p[i].y));
    }}
    return bad;
}}

// ‖y − x‖_phys (REQ-ENC-029), x the reference: the largest of |Δm|, ‖Δr‖/R_x and ‖Δp‖/P_x, the extents floored at 1.
// Where `bad`, every value is replaced by 0 by a select before the arithmetic, never measured (R-436).
fn roundtrip_phys(x: FragDecode, y: FragFree, bad: bool) -> f32 {{
    let z2 = vec2<f32>(0.0);
    let k2 = vec2<bool>(bad);
    let dm = select(y.m - x.m, vec3<f32>(0.0), vec3<bool>(bad));
    var r_x = 1.0;
    var p_x = 1.0;
    for (var i = 0u; i < 3u; i++) {{
        r_x = max(r_x, length(select(x.r[i], z2, k2)));
        p_x = max(p_x, length(select(x.p[i], z2, k2)));
    }}
    var worst = max(max(abs(dm.x), abs(dm.y)), abs(dm.z));
    for (var i = 0u; i < 3u; i++) {{
        worst = max(worst, length(select(y.r[i], z2, k2) - select(x.r[i], z2, k2)) / r_x);
        worst = max(worst, length(select(y.p[i], z2, k2) - select(x.p[i], z2, k2)) / p_x);
    }}
    return worst;
}}

// The class (render::presets::roundtrip::Class) of a trip, the first that holds: 4 invalid, 1 either decode DEGENERATE,
// 2 lookup_clamped, 3 lookup_mirrored, else 0.
fn roundtrip_class(invalid: bool, tag: u32, tag_free: u32, notices: u32) -> u32 {{
    if (invalid) {{
        return 4u;
    }}
    if (tag != 0u || tag_free != 0u) {{
        return 1u;
    }}
    if ((notices & {CLAMPED:#x}u) != 0u) {{
        return 2u;
    }}
    if ((notices & {MIRRORED}u) != 0u) {{
        return 3u;
    }}
    return 0u;
}}

// The trip from the chart's decode d at the pixel: the encode of its state, the free decode of that z, the residual
// and the class.
fn roundtrip_of(d: FragDecode) -> RoundTrip {{
    let e = frag_encode(d.m, d.r, d.p);
    let back = frag_decode_free(e.z);
    var o: RoundTrip;
    o.notices = e.notices;
    o.tags = d.tag | (back.tag << 8u);
    let bad = roundtrip_bad(d.m, d.r, d.p) || roundtrip_bad(back.m, back.r, back.p);
    let residual = roundtrip_phys(d, back, bad);
    let invalid = bad || preset_non_finite(bitcast<u32>(residual));
    o.ratio = select(residual / ROUNDTRIP_EPS_PHYS, 0.0, invalid);
    o.kind = roundtrip_class(invalid, d.tag, back.tag, e.notices);
    return o;
}}

// The trip's colour: red past ε_phys untagged, black within it; a tag's hue, no red; the hatch where invalid.
fn roundtrip_colour(t: RoundTrip, frag_xy: vec2<f32>) -> vec3<f32> {{
    let b = min(log2(max(t.ratio, 1.0)) / ROUNDTRIP_SPAN, 1.0);
    let s = ROUNDTRIP_FLOOR + (1.0 - ROUNDTRIP_FLOOR) * b;
    switch t.kind {{
        case 0u: {{ return vec3<f32>(b, 0.0, 0.0); }}
        case 1u: {{ return vec3<f32>(0.0, s, s); }}
        case 2u: {{ return vec3<f32>(0.0, 0.0, s); }}
        case 3u: {{ return vec3<f32>(0.0, s, 0.0); }}
        default: {{ return debug_invalid(frag_xy); }}
    }}
}}
",
        eps = eps_phys() as f32,
    )
}

/// The view.
const VIEW: &str = "
fn view(rc: RenderContext, l0: Lanes) -> vec4<f32> {
    let d = frag_decode(l0.chart.slice_uv);
    var l = l0;
    l.chart.z = d.z;
    let frag_xy = vec2<f32>(l.screen.pixel) + vec2<f32>(0.5);
    return vec4<f32>(roundtrip_colour(roundtrip_of(d), frag_xy), 1.0);
}
";

/// The words view: `(ratio's bits, class, notices, tags)`, `tags` the first decode's tag word and the free decode's
/// shifted by 8, which the tests and the golden suite's checks read.
const WORDS: &str = "
fn view(rc: RenderContext, l: Lanes) -> vec4<u32> {
    let t = roundtrip_of(frag_decode(l.chart.slice_uv));
    return vec4<u32>(bitcast<u32>(t.ratio), t.kind, t.notices, t.tags);
}
";

/// The snippets the trip calls beyond the chart's decode: the free decode's section and the encode.
fn stages() -> String {
    let free = section(DECODE_WGSL, "free")
        .unwrap_or_else(|e| panic!("the generated decode is the translation's: {e}"));
    format!("{free}\n{ENCODE_WGSL}\n")
}

/// The WGSL every ROUNDTRIP view places after the chart's decode: the shared bit test, the free decode, the encode,
/// and the trip's functions (`roundtrip_of`, `roundtrip_class`, `roundtrip_colour`), each view's `fn view` after it.
pub fn snippet() -> String {
    format!("{SHARED_WGSL}{}{}", stages(), trip())
}

/// The ROUNDTRIP preset, locked.
pub fn preset() -> Preset {
    Preset {
        name: NAME,
        view: format!("{}{VIEW}", snippet()),
        output: ViewOutput::Colour,
    }
}

/// The ROUNDTRIP preset's trip as words, `(ratio, class, notices, tags)` per pixel, for the tests and the golden suite.
pub fn words() -> Preset {
    Preset {
        name: NAME,
        view: format!("{}{WORDS}", snippet()),
        output: ViewOutput::Words,
    }
}
