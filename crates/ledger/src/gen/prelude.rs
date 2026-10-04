//! The shared prelude (render_gui_spec §10.1; render contract Part 2; lowering Part 3a): the fixed library every
//! fragment node may call, built-in, debug or custom, emitted from the ledger as the rest of the fragment side's
//! generated WGSL is, into `crates/render/shaders/wgsl/lib/prelude.wgsl` ([`PATH`]). It holds:
//! - the tier's baked features, `const has_ftle` and `const has_word`, from the [`Tier`] the stored variant and the
//!   word binding give (lowering Part 3a); and `has_ensemble()`, read from a uniform, never baked, so toggling E never
//!   re-bakes (R-145). The uniform is bound in group 0, the assembler's per-frame uniforms (R-343), at the numbers of
//!   [`uniforms_binding`], written as `PRELUDE_UNIFORMS_GROUP` and `PRELUDE_UNIFORMS_BINDING`;
//! - `is_absent_nan(x)`, the absence test by bits against the canonical quiet NaN ([`crate::payload::canonical_qnan_bits`];
//!   lowering Part 3a), never `isnan` (R-114, R-297);
//! - `range_norm` (render_gui_spec §10.1), its `auto` argument named `auto_range`, `auto` being a WGSL reserved word;
//! - the colour-space maps the ramps need: `srgb_to_linear`, `linear_to_srgb`, `oklab_to_linear` and
//!   `oklch_to_linear` (dd_colouring §3.1);
//! - the ramps the M1 views need, each returning linear RGB, the colour slot's space (render contract Part 2):
//!   `ramp_viridis` and `ramp_twilight` from matplotlib's published tables (R-122, [`luts`]), `ramp_grey` and
//!   `hue_wheel`;
//! - `debug_invalid(frag_xy)`, the hatched invalid pattern (R-132, R-136), its pattern and colours [`hatch`]'s, a
//!   calibration proposed here (REQ-COL-055, R-71).
//!
//! The checked-in file is the full tier's ([`Tier::FULL`]), as the unpack layer and the read side are; the assembler
//! emits the prelude at its own tier with [`wgsl`]. The prelude reads neither stored buffer: only the read side's
//! `sample_read` does (R-343, R-378).

use std::fmt::Write as _;
use std::path::PathBuf;

use crate::gen::read::Tier;
use crate::gen::Generated;

/// Where the prelude is written, relative to the workspace root.
pub const PATH: &str = "crates/render/shaders/wgsl/lib/prelude.wgsl";

/// One published colour-map table (R-122): its name, the WGSL names the prelude gives its data and its ramp, and its
/// stops as the data file writes them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lut {
    /// The colour map's name, as matplotlib's.
    pub name: &'static str,
    /// The WGSL constant holding the stops.
    pub constant: &'static str,
    /// The WGSL ramp reading them.
    pub ramp: &'static str,
    /// The data file's text: `#` comment lines naming the source, then one stop per line, its sRGB-encoded red, green
    /// and blue in [0, 1] as the source writes them.
    pub data: &'static str,
}

impl Lut {
    /// The stops, each as the three decimal literals the source writes, in order.
    pub fn literals(&self) -> Vec<[&'static str; 3]> {
        self.data
            .lines()
            .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
            .map(|l| {
                let mut it = l.split_whitespace();
                let mut next = || it.next().unwrap_or("missing");
                [next(), next(), next()]
            })
            .collect()
    }

    /// The stops as numbers; a literal that is not a number reads as NaN, so no stop is dropped silently.
    pub fn stops(&self) -> Vec<[f64; 3]> {
        self.literals()
            .into_iter()
            .map(|s| s.map(|x| x.parse().unwrap_or(f64::NAN)))
            .collect()
    }
}

/// The published tables the prelude's LUT ramps read (R-122): matplotlib 3.8.0's `_viridis_data` and
/// `_twilight_data`, transcribed verbatim into `crates/ledger/data/lut/`, each file naming its source.
pub fn luts() -> [Lut; 2] {
    [
        Lut {
            name: "viridis",
            constant: "LUT_VIRIDIS",
            ramp: "ramp_viridis",
            data: include_str!("../../data/lut/viridis.txt"),
        },
        Lut {
            name: "twilight",
            constant: "LUT_TWILIGHT",
            ramp: "ramp_twilight",
            data: include_str!("../../data/lut/twilight.txt"),
        },
    ]
}

/// The prelude's uniform block's bind group and binding number: group 0, the assembler's per-frame uniforms (R-343),
/// binding 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UniformsBinding {
    pub group: u32,
    pub binding: u32,
}

/// Where the prelude's uniform block, `PreludeUniforms`, is bound (applied per R-369: the corpus puts the per-frame
/// uniforms in group 0 and gives no binding within it).
pub const fn uniforms_binding() -> UniformsBinding {
    UniformsBinding {
        group: 0,
        binding: 0,
    }
}

/// The words of the prelude's uniform block for an ensemble of `e` copies: `has_ensemble`, 1 when `e ≥ 1` and 0 at
/// E = 0, then three words of padding to the uniform block's 16 bytes (R-145: a computed spread is a real value only
/// when E ≥ 1, lowering Part 3a). The host writes it each frame; the fragment variant does not change with `e`.
pub fn uniform_words(e: u32) -> [u32; 4] {
    [u32::from(e >= 1), 0, 0, 0]
}

/// The invalid hatch (REQ-COL-055; R-132, R-136): diagonal stripes, each `half_period` pixels wide across `x + y`,
/// alternating `colours[0]` and `colours[1]`, given as 8-bit sRGB. Proposed, R-71: the human confirms it at the M1
/// gate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hatch {
    pub colours: [[u8; 3]; 2],
    /// The stripe width in pixels, a power of two: the pattern shifts `x + y` right by its log2.
    pub half_period: u32,
}

/// The proposed hatch (proposed, R-71; REQ-COL-055): violet `#9B00FF` and aquamarine `#50FFD2`, in stripes 4 px wide.
/// Each colour is farther in OKLab from every palette entry (the outcome palette, the `dbg_*` palettes, the LUTs, the
/// grey ramp and the OKLCH hue circle at L 0.75, C 0.12) than the flat magenta R-16 kept is; the evidence is the PR's
/// and `render/tests/prelude.rs`'s.
pub const fn hatch() -> Hatch {
    Hatch {
        colours: [[0x9b, 0x00, 0xff], [0x50, 0xff, 0xd2]],
        half_period: 4,
    }
}

/// `hue_wheel`'s OKLCH lightness and chroma, `(L, C)`: the lightness at which the sRGB gamut holds the largest chroma
/// at every hue, 0.1275 at L 0.75, less a margin (R-72: the corpus names `hue_wheel` and does not define it).
pub const fn hue_wheel_lc() -> (f64, f64) {
    (0.75, 0.12)
}

/// The full tier's prelude, the checked-in file.
pub fn emit() -> Generated {
    Generated {
        path: PathBuf::from(PATH),
        contents: wgsl(Tier::FULL),
    }
}

/// `x` as a WGSL f32 literal that reads back as `x` (Rust's shortest round-trip form, with a decimal point).
fn float(x: f64) -> String {
    let s = format!("{:?}", x as f32);
    if s.contains('.') || s.contains('e') {
        s
    } else {
        format!("{s}.0")
    }
}

/// The prelude at `tier`.
pub fn wgsl(tier: Tier) -> String {
    let b = uniforms_binding();
    let qnan = crate::payload::canonical_qnan_bits();
    let h = hatch();
    let (wheel_l, wheel_c) = hue_wheel_lc();
    let mut out = format!(
        r"// Generated by `cargo xtask codegen` from the layout table (`crates/ledger/src/payload.rs`) and the published
// colour-map tables (`crates/ledger/data/lut/`); do not edit.
// The shared prelude (render_gui_spec §10.1): the fixed library any fragment node may call, built-in, debug or
// custom. It reads neither stored buffer; only the read side's `sample_read` does (R-343, R-378).

// This variant's tier (lowering Part 3a): baked, dead-code-eliminated, reliable under fast-math. `has_ftle` is whether
// the stored variant is `SimStateFTLE`; `has_word` whether the word buffer is bound.
const has_ftle: bool = {has_ftle};
const has_word: bool = {has_word};

// The prelude's per-frame uniforms, in group 0, the assembler's (R-343). `has_ensemble` is 1 when E ≥ 1 and 0 at
// E = 0: a uniform, not a bake, so toggling E never re-bakes the variant (R-145).
const PRELUDE_UNIFORMS_GROUP: u32 = {group}u;
const PRELUDE_UNIFORMS_BINDING: u32 = {binding}u;
struct PreludeUniforms {{
    has_ensemble: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}}
@group({group}) @binding({binding}) var<uniform> prelude_uniforms: PreludeUniforms;

// Whether the ensemble is on (E ≥ 1), read from the uniform (R-145). At E = 0 the read side's `ensemble_spread` is the
// canonical quiet NaN (lowering Part 3a).
fn has_ensemble() -> bool {{ return prelude_uniforms.has_ensemble != 0u; }}

// Whether `x` is the absence sentinel, the canonical quiet NaN: an exact bit comparison, reliable where `isnan` is
// not under fast-math (lowering Part 3a; R-114, R-297).
fn is_absent_nan(x: f32) -> bool {{ return bitcast<u32>(x) == {qnan:#010x}u; }}

// Min–max normalisation to [0, 1] for ramp lookup (render_gui_spec §10.1): clamp((x − l)/(h − l), 0, 1) with
// (l, h) = select((lo, hi), meas, auto_range); `auto` is a WGSL reserved word. A degenerate range, h = l, reads 0
// (R-72). `meas.x` is the measured low, `meas.y` the high.
fn range_norm(x: f32, lo: f32, hi: f32, auto_range: bool, meas: vec2<f32>) -> f32 {{
    let l = select(lo, meas.x, auto_range);
    let h = select(hi, meas.y, auto_range);
    return select(clamp((x - l) / (h - l), 0.0, 1.0), 0.0, h == l);
}}

// The sRGB transfer, per channel (dd_colouring §3.1): encoded to linear.
fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {{
    return select(pow((c + 0.055) / 1.055, vec3<f32>(2.4)), c / 12.92, c <= vec3<f32>(0.04045));
}}

// The sRGB transfer's inverse, per channel (dd_colouring §3.1): linear to encoded.
fn linear_to_srgb(c: vec3<f32>) -> vec3<f32> {{
    return select(1.055 * pow(c, vec3<f32>(1.0 / 2.4)) - 0.055, 12.92 * c, c <= vec3<f32>(0.0031308));
}}

// OKLab to linear sRGB (dd_colouring §3.1): lms' = M₂⁻¹·Lab, cubed, then M₁⁻¹.
fn oklab_to_linear(lab: vec3<f32>) -> vec3<f32> {{
    let l_ = lab.x + 0.3963377774 * lab.y + 0.2158037573 * lab.z;
    let m_ = lab.x - 0.1055613458 * lab.y - 0.0638541728 * lab.z;
    let s_ = lab.x - 0.0894841775 * lab.y - 1.2914855480 * lab.z;
    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;
    return vec3<f32>(
        4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
        -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
        -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s,
    );
}}

// OKLCH to linear sRGB, the hue `turns` in turns: reduced to [−0.5, 0.5) first, so the angle passed to cos and sin
// lies in [−π, π), where WGSL bounds their error.
fn oklch_to_linear(l: f32, c: f32, turns: f32) -> vec3<f32> {{
    let f = fract(turns);
    let h = 6.2831855 * (f - select(0.0, 1.0, f >= 0.5));
    return oklab_to_linear(vec3<f32>(l, c * cos(h), c * sin(h)));
}}

// The greyscale ramp: OKLab (L = t, 0, 0), `t` clamped to [0, 1], the colour slot's None occupant's grey
// (colour_composition §4.1). Linear RGB.
fn ramp_grey(t: f32) -> vec3<f32> {{ return oklab_to_linear(vec3<f32>(clamp(t, 0.0, 1.0), 0.0, 0.0)); }}

// The hue wheel: OKLCH at L {wheel_l}, C {wheel_c}, the hue `t` turns, so t and t + 1 are one colour (R-72). In gamut at
// every hue. Linear RGB.
fn hue_wheel(t: f32) -> vec3<f32> {{ return oklch_to_linear({wl}, {wc}, t); }}

// The reserved invalid rendering (R-132, R-136; REQ-COL-055, proposed, R-71): a hatch drawn from the pixel position,
// diagonal stripes {half} px wide across x + y, alternating #{a0:02X}{a1:02X}{a2:02X} and #{b0:02X}{b1:02X}{b2:02X}. It collides with no palette
// entry. Only NaN gets it; a stored sentinel shows its value (R-136). Linear RGB.
fn debug_invalid(frag_xy: vec2<f32>) -> vec3<f32> {{
    let p = vec2<i32>(floor(frag_xy));
    let stripe = ((p.x + p.y) >> {shift}u) & 1;
    let a = vec3<f32>({a0}.0, {a1}.0, {a2}.0) / 255.0;
    let b = vec3<f32>({b0}.0, {b1}.0, {b2}.0) / 255.0;
    return srgb_to_linear(select(a, b, stripe == 1));
}}
",
        has_ftle = tier.has_ftle,
        has_word = tier.has_word,
        group = b.group,
        binding = b.binding,
        wl = float(wheel_l),
        wc = float(wheel_c),
        half = h.half_period,
        shift = h.half_period.trailing_zeros(),
        a0 = h.colours[0][0],
        a1 = h.colours[0][1],
        a2 = h.colours[0][2],
        b0 = h.colours[1][0],
        b1 = h.colours[1][1],
        b2 = h.colours[1][2],
    );
    for lut in luts() {
        out.push_str(&lut_wgsl(&lut));
    }
    out
}

/// `lut`'s stops as a WGSL constant, and its ramp: the stops at `t = k/(N − 1)`, linear interpolation between them in
/// the table's sRGB-encoded space, then decoded to linear RGB; `t` clamped to [0, 1]. A stop's literal is the
/// source's, so the constant holds the published value rounded once, to f32.
fn lut_wgsl(lut: &Lut) -> String {
    let stops = lut.literals();
    let n = stops.len();
    let mut out = format!(
        "\n// matplotlib's published `{name}` table (R-122), {n} stops, sRGB-encoded: {data}.\n\
         const {c}: array<vec3<f32>, {n}> = array<vec3<f32>, {n}>(\n",
        name = lut.name,
        data = format_args!("crates/ledger/data/lut/{}.txt", lut.name),
        c = lut.constant,
    );
    for s in &stops {
        let _ = writeln!(out, "    vec3<f32>({}, {}, {}),", s[0], s[1], s[2]);
    }
    let _ = write!(
        out,
        ");\n\n// `{name}` at `t` in [0, 1] (clamped), the stops at k/{last}, interpolated in sRGB then decoded: linear RGB.\n\
         fn {ramp}(t: f32) -> vec3<f32> {{\n    \
         let x = clamp(t, 0.0, 1.0) * {last}.0;\n    \
         let k = min(u32(x), {below}u);\n    \
         return srgb_to_linear(mix({c}[k], {c}[k + 1u], x - f32(k)));\n}}\n",
        name = lut.name,
        ramp = lut.ramp,
        c = lut.constant,
        last = n.saturating_sub(1),
        below = n.saturating_sub(2),
    );
    out
}
