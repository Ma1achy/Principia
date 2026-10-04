//! qa's tests for TASK-M1-03, written from the requirements it closes and the definitions their sources give, never
//! from the implementation's own mirror (`render::present` is not used here):
//! - REQ-RENDER-020 (render_gui_spec §10.1): the prelude is the layout build step's output, not a hand file, and a
//!   custom node calling `ramp_viridis` (and the prelude's other members) compiles after it; the ramps match
//!   matplotlib's published tables at their stops (R-122).
//! - REQ-RENDER-021 (render_gui_spec §10.1): `range_norm` is `clamp((x − l)/(h − l), 0, 1)`, `(l, h)` the fixed range
//!   or the measured one by `auto`, evaluated on the GPU on fixtures whose results are exact in f32.
//! - REQ-TOOL-009, REQ-TOOL-122 (render contract Part 5, "Presentation layer", its renderings): each `dbg_*` helper
//!   returns its defined colour on fixture inputs; `dbg_sentinel(−1.0, p)` is −1.0's place on the ramp; the absence
//!   NaN's bits draw `debug_invalid(p)`, the hatch, and nothing else does (R-136).
//! - REQ-COL-055 (R-132, R-136): `debug_invalid` draws the proposed hatch from the pixel position, and neither of its
//!   colours comes as close to any palette entry as the flat magenta R-132 ruled a collision does to `#E034C6`. The
//!   palettes include every LUT of colour_composition §7.1 (render contract Part 5, the hatch), between its stops too.
//! - Lowering Part 3a, R-297: the absence test is by bits; it holds on the GPU for the canonical quiet NaN (the
//!   sentinel checks below), where an `isnan` under fast-math could not be relied on.
//!
//! Every colour is read back from the GPU. A colour the corpus gives as 8-bit sRGB is compared exactly at 8 bits; a
//! colour defined by a formula or by the published float stops is compared in sRGB encoding to half an 8-bit code,
//! the precision of the display the helpers draw for (no tolerance is chosen here beyond that). The published stops
//! below are transcribed from matplotlib's `_cm_listed.py` (`_viridis_data`, `_twilight_data`), the source R-122 names.
//!
//! Each check takes the specification it tests as an argument; its control (R-176) passes a deliberately wrong one
//! (a swapped colour, a shifted constant, an inverted selection) and must fail.

use std::path::Path;

use ledger::gen::{self, prelude};
use validation::gpu::{BindingKind, GpuHarness};
use validation::negative_control;

type Rgb = [f64; 3];

// ── the specification, from the docs ──────────────────────────────────────────────────────────────────────────────

/// The values the render contract's presentation layer (R-72; REQ-TOOL-122) and the hatch proposal (REQ-COL-055)
/// define, and lowering Part 3a's absence NaN.
#[derive(Clone, Copy, Debug)]
struct Spec {
    /// Okabe–Ito, 8-bit sRGB, published order.
    okabe_ito: [[u8; 3]; 8],
    /// `dbg_flag(true)`, `dbg_flag(false)`.
    flag: [[u8; 3]; 2],
    /// `dbg_cat`'s golden-angle OKLCH lightness and chroma.
    golden_lc: (f64, f64),
    /// The golden-angle step in turns, `φ_g = (√5 − 1)/2`.
    phi_g: f64,
    /// The hatch's colours, stripe 0 then stripe 1, 8-bit sRGB.
    hatch: [[u8; 3]; 2],
    /// The hatch's stripe width in pixels, across `x + y`.
    hatch_width: i64,
    /// `hue_wheel`'s OKLCH lightness and chroma (render_gui_spec §10.1, as built).
    wheel_lc: (f64, f64),
    /// `range_norm` takes the measured range when `auto` is true.
    auto_selects_meas: bool,
}

const SPEC: Spec = Spec {
    okabe_ito: [
        [0x00, 0x00, 0x00],
        [0xE6, 0x9F, 0x00],
        [0x56, 0xB4, 0xE9],
        [0x00, 0x9E, 0x73],
        [0xF0, 0xE4, 0x42],
        [0x00, 0x72, 0xB2],
        [0xD5, 0x5E, 0x00],
        [0xCC, 0x79, 0xA7],
    ],
    flag: [[0x00, 0x9E, 0x73], [0xD5, 0x5E, 0x00]],
    golden_lc: (0.75, 0.12),
    phi_g: 0.618_033_988_749_894_8,
    hatch: [[0x9B, 0x00, 0xFF], [0x48, 0xFF, 0xFF]],
    hatch_width: 4,
    wheel_lc: (0.75, 0.12),
    auto_selects_meas: true,
};

/// colour_composition §1.4's outcome-state palette, `#E034C6` (body 1 escape) among it.
const OUTCOME: [u32; 9] = [
    0xDE2D2D, 0x2EBC4E, 0x3462E0, 0x141418, 0xECECF0, 0xF0DE32, 0xE034C6, 0x30C8DC, 0xF29620,
];

/// matplotlib's `_viridis_data`, the stops these tests read, by index (256 stops).
const VIRIDIS: [(usize, Rgb); 11] = [
    (0, [0.267004, 0.004874, 0.329415]),
    (63, [0.231674, 0.318106, 0.544834]),
    (64, [0.229739, 0.322361, 0.545706]),
    (127, [0.128729, 0.563265, 0.551229]),
    (128, [0.127568, 0.566949, 0.550556]),
    (191, [0.360741, 0.785964, 0.387814]),
    (192, [0.369214, 0.788888, 0.382914]),
    (251, [0.9553, 0.901065, 0.118128]),
    (252, [0.964894, 0.902323, 0.123941]),
    (254, [0.983868, 0.904867, 0.136897]),
    (255, [0.993248, 0.906157, 0.143936]),
];

/// matplotlib's `_twilight_data`, the stops these tests read, by index (510 stops).
const TWILIGHT: [(usize, Rgb); 6] = [
    (
        0,
        [0.8857501584075443, 0.8500092494306783, 0.8879736506427196],
    ),
    (
        1,
        [0.8837852019553906, 0.8507294054031063, 0.8872322209694989],
    ),
    (
        254,
        [
            0.18739228342697645,
            0.07710209689958833,
            0.21618875376309582,
        ],
    ),
    (
        255,
        [
            0.18488035509396164,
            0.07942573027972388,
            0.21307651648984993,
        ],
    ),
    (
        508,
        [0.8855471481195238, 0.8498717428363158, 0.8833620612117095],
    ),
    (
        509,
        [0.8857115512284565, 0.8500218611585632, 0.8857253899008712],
    ),
];

// ── colour arithmetic (dd_colouring §3.1) ─────────────────────────────────────────────────────────────────────────

fn decode(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn encode(c: f64) -> f64 {
    if c <= 0.0031308 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

fn from8(c: [u8; 3]) -> Rgb {
    c.map(|x| decode(f64::from(x) / 255.0))
}

fn hex(h: u32) -> Rgb {
    from8([(h >> 16) as u8, (h >> 8) as u8, h as u8])
}

/// A linear colour as the 8-bit sRGB code it displays as.
fn to8(c: Rgb) -> [i64; 3] {
    c.map(|x| (encode(x.clamp(0.0, 1.0)) * 255.0).round() as i64)
}

fn oklab_to_linear(lab: Rgb) -> Rgb {
    let [l, a, b] = lab;
    let l_ = l + 0.3963377774 * a + 0.2158037573 * b;
    let m_ = l - 0.1055613458 * a - 0.0638541728 * b;
    let s_ = l - 0.0894841775 * a - 1.2914855480 * b;
    let (l, m, s) = (l_ * l_ * l_, m_ * m_ * m_, s_ * s_ * s_);
    [
        4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
        -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
        -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s,
    ]
}

fn linear_to_oklab(c: Rgb) -> Rgb {
    let [r, g, b] = c;
    let l = (0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b).cbrt();
    let m = (0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b).cbrt();
    let s = (0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b).cbrt();
    [
        0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
        1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
        0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
    ]
}

fn oklch(l: f64, c: f64, turns: f64) -> Rgb {
    let h = std::f64::consts::TAU * turns;
    oklab_to_linear([l, c * h.cos(), c * h.sin()])
}

fn dist(a: Rgb, b: Rgb) -> f64 {
    let (a, b) = (linear_to_oklab(a), linear_to_oklab(b));
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// A published stop, encoded sRGB, from `table`.
fn stop(table: &[(usize, Rgb)], k: usize) -> Rgb {
    table
        .iter()
        .find(|(i, _)| *i == k)
        .unwrap_or_else(|| panic!("stop {k} is not transcribed here"))
        .1
}

/// A LUT ramp of `n` stops at `t` (clamped), linear between stops in the table's sRGB encoding, decoded
/// (render_gui_spec §10.1, as built; R-122).
fn lut(table: &[(usize, Rgb)], n: usize, t: f64) -> Rgb {
    let x = t.clamp(0.0, 1.0) * (n - 1) as f64;
    let k = (x.floor() as usize).min(n - 2);
    let f = x - k as f64;
    let (a, b) = if f == 0.0 {
        let a = stop(table, k);
        (a, a)
    } else {
        (stop(table, k), stop(table, k + 1))
    };
    [0, 1, 2].map(|c| decode(a[c] + (b[c] - a[c]) * f))
}

fn viridis(t: f64) -> Rgb {
    lut(&VIRIDIS, 256, t)
}

/// The GPU's colour within half an 8-bit code of `want` in every encoded channel, or a panic naming `what`.
fn near(what: &str, got: Rgb, want: Rgb) {
    for c in 0..3 {
        let (g, w) = (encode(got[c]), encode(want[c]));
        assert!(
            (g - w).abs() <= 0.5 / 255.0,
            "{what}: channel {c} encodes to {g:.6}, the definition gives {w:.6} (more than half an 8-bit code off); \
             got {got:?}, want {want:?}"
        );
    }
}

/// The GPU's colour displays as exactly the 8-bit sRGB colour `want`, or a panic naming `what`.
fn exact8(what: &str, got: Rgb, want: [u8; 3]) {
    let want_i = want.map(i64::from);
    assert_eq!(
        to8(got),
        want_i,
        "{what}: displays as {:?}, the definition gives {want_i:?}",
        to8(got)
    );
}

// ── the GPU evaluation ─────────────────────────────────────────────────────────────────────────────────────────────

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn read(rel: &str) -> String {
    let path = root().join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

const PRESENT: &str = "crates/render/shaders/wgsl/lib/present.wgsl";

/// The shared library as a fragment node sees it: the checked-in prelude, then the presentation layer.
fn library() -> String {
    format!("{}\n{}", read(prelude::PATH), read(PRESENT))
}

const W: usize = 64;

/// qa's entries: `qa_case` evaluates case `qa_in[2i], qa_in[2i + 1]` at pixel `i`; `qa_hatch` draws
/// `debug_invalid` at each pixel's own position; `qa_sentinel` draws `dbg_sentinel(qa_in[0].x as f32, pos)`.
const ENTRY: &str = r"
@group(1) @binding(0) var<storage, read> qa_in: array<vec4<u32>>;

fn qa_rgb(c: vec3<f32>) -> vec4<u32> { return vec4<u32>(bitcast<vec3<u32>>(c), 0u); }
fn qa_f(w: u32) -> f32 { return bitcast<f32>(w); }

@fragment
fn qa_case(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<u32> {
    let i = u32(pos.y) * 64u + u32(pos.x);
    let a = qa_in[2u * i];
    let b = qa_in[2u * i + 1u];
    let x = qa_f(a.y);
    switch a.x {
        case 1u: {
            let t = range_norm(x, qa_f(a.z), qa_f(a.w), b.x != 0u, vec2<f32>(qa_f(b.y), qa_f(b.z)));
            return vec4<u32>(bitcast<u32>(t), 0u, 0u, 0u);
        }
        case 2u: { return qa_rgb(dbg_cat(a.y, a.z)); }
        case 3u: { return qa_rgb(dbg_lin(x, qa_f(a.z), qa_f(a.w))); }
        case 4u: { return qa_rgb(dbg_log(x, qa_f(a.z))); }
        case 5u: { return qa_rgb(dbg_flag(a.y != 0u)); }
        case 6u: { return qa_rgb(dbg_hash_u32(a.y)); }
        case 7u: { return qa_rgb(dbg_sentinel(x, vec2<f32>(qa_f(b.x), qa_f(b.y)))); }
        case 8u: { return qa_rgb(debug_invalid(vec2<f32>(qa_f(b.x), qa_f(b.y)))); }
        case 9u: { return qa_rgb(ramp_viridis(x)); }
        case 10u: { return qa_rgb(ramp_twilight(x)); }
        case 11u: { return qa_rgb(ramp_grey(x)); }
        case 12u: { return qa_rgb(hue_wheel(x)); }
        default: { return vec4<u32>(0xffffffffu); }
    }
}

@fragment
fn qa_hatch(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<u32> {
    return qa_rgb(debug_invalid(pos.xy));
}

@fragment
fn qa_sentinel(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<u32> {
    return qa_rgb(dbg_sentinel(qa_f(qa_in[0].x), pos.xy));
}
";

const RANGE_NORM: u32 = 1;
const CAT: u32 = 2;
const LIN: u32 = 3;
const LOG: u32 = 4;
const FLAG: u32 = 5;
const HASH: u32 = 6;
const SENTINEL: u32 = 7;
const INVALID: u32 = 8;
const VIRIDIS_OP: u32 = 9;
const TWILIGHT_OP: u32 = 10;
const GREY_OP: u32 = 11;
const WHEEL_OP: u32 = 12;

type Case = [u32; 8];

fn case(op: u32, a: &[u32], b: &[u32]) -> Case {
    let mut c = [0u32; 8];
    c[0] = op;
    c[1..1 + a.len()].copy_from_slice(a);
    c[4..4 + b.len()].copy_from_slice(b);
    c
}

fn fb(x: f32) -> u32 {
    x.to_bits()
}

fn gpu() -> GpuHarness {
    GpuHarness::new().expect("a GPU device")
}

fn module() -> String {
    format!("{}\n{ENTRY}", library())
}

/// Draws `entry` over a `w` × `h` target with `words` in `qa_in`; the pixels, row by row.
fn draw(gpu: &GpuHarness, entry: &str, words: &[u32], w: u32, h: u32) -> Vec<[u32; 4]> {
    let kernel = gpu
        .fragment(
            &module(),
            entry,
            &[&[BindingKind::Uniform], &[BindingKind::Storage]],
            w,
            h,
        )
        .unwrap_or_else(|e| panic!("{e}"));
    kernel
        .draw(&[&[&[0, 0, 0, 0]], &[words]])
        .unwrap_or_else(|e| panic!("{e}"))
}

/// Each case's four result words, evaluated on the GPU.
fn eval(gpu: &GpuHarness, cases: &[Case]) -> Vec<[u32; 4]> {
    let rows = cases.len().div_ceil(W);
    let mut words: Vec<u32> = cases.iter().flatten().copied().collect();
    words.resize(rows * W * 8, 0);
    let px = draw(gpu, "qa_case", &words, W as u32, rows as u32);
    px[..cases.len()].to_vec()
}

fn rgb(p: [u32; 4]) -> Rgb {
    [0, 1, 2].map(|c| f64::from(f32::from_bits(p[c])))
}

fn eval_rgb(gpu: &GpuHarness, cases: &[Case]) -> Vec<Rgb> {
    eval(gpu, cases).into_iter().map(rgb).collect()
}

// ── REQ-RENDER-020: the prelude is generated, and a custom node calling it compiles ───────────────────────────────

/// `on_disk` is what the layout build step (`ledger::gen::generate` over its emitters) writes to `prelude::PATH`, and
/// it says so.
fn check_generated(on_disk: &str) {
    let files = gen::generate(&ledger::layout(), gen::EMITTERS).expect("the layout generates");
    let written: Vec<_> = files
        .iter()
        .filter(|g| {
            g.path
                .ends_with("crates/render/shaders/wgsl/lib/prelude.wgsl")
        })
        .collect();
    assert_eq!(
        written.len(),
        1,
        "the layout build step writes the prelude once"
    );
    assert!(
        written[0].contents == on_disk,
        "the checked-in prelude differs from the layout build step's output"
    );
    assert!(
        on_disk.lines().take(3).any(|l| l.contains("Generated")),
        "the checked-in prelude does not say it is generated"
    );
}

#[test]
fn qa_prelude_generated_by_the_layout_step() {
    check_generated(&read(prelude::PATH));
}

negative_control!(
    qa_prelude_generated_by_the_layout_step,
    "a hand edit of the checked-in prelude must be caught",
    expected = "differs from the layout build step's output",
    check_generated(&read(prelude::PATH).replacen("fn ", "fn  ", 1))
);

/// A custom node, written as a user would, calling the prelude's ramps, value mapping, colour-space map and reserved
/// rendering, and an entry drawing it.
const CUSTOM: &str = r"
fn my_custom_colour(x: f32, lo: f32, hi: f32, frag_xy: vec2<f32>) -> vec3<f32> {
    let t = range_norm(x, lo, hi, false, vec2<f32>(0.0, 1.0));
    let c = 0.25 * ramp_viridis(t) + 0.25 * ramp_twilight(t) + 0.25 * ramp_grey(t) + 0.25 * hue_wheel(t);
    return select(c, debug_invalid(frag_xy), x > hi) + oklab_to_linear(vec3<f32>(0.0, 0.0, 0.0));
}

fn my_viridis_only(x: f32) -> vec3<f32> { return ramp_viridis(x); }

@fragment
fn my_custom(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<u32> {
    let c = my_custom_colour(pos.x, 0.0, 64.0, pos.xy) + my_viridis_only(pos.x / 64.0);
    return vec4<u32>(bitcast<vec3<u32>>(c), 1u);
}
";

/// At every tier, `node` after the prelude at that tier and the presentation layer validates with naga and compiles
/// to a GPU pipeline that draws.
fn check_custom_node(gpu: &GpuHarness, node: &str) {
    for tier in gen::read::Tier::ALL {
        let source = format!("{}\n{}\n{node}", prelude::wgsl(tier), read(PRESENT));
        let m = naga::front::wgsl::parse_str(&source).unwrap_or_else(|e| {
            panic!(
                "a custom node does not compile at {tier:?}: {}",
                e.emit_to_string(&source)
            )
        });
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&m)
        .unwrap_or_else(|e| {
            panic!(
                "a custom node does not compile at {tier:?}: {}",
                e.emit_to_string(&source)
            )
        });
        let kernel = gpu
            .fragment(&source, "my_custom", &[&[BindingKind::Uniform]], 4, 1)
            .unwrap_or_else(|e| panic!("a custom node does not compile at {tier:?}: {e}"));
        let px = kernel
            .draw(&[&[&[0, 0, 0, 0]]])
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(px.iter().all(|p| p[3] == 1), "the custom node did not draw");
    }
}

#[test]
fn qa_prelude_generated_custom_node_compiles() {
    check_custom_node(&gpu(), CUSTOM);
}

negative_control!(
    qa_prelude_generated_custom_node_compiles,
    "a custom node calling a ramp the prelude lacks must not compile",
    expected = "a custom node does not compile",
    check_custom_node(
        &gpu(),
        &CUSTOM.replace("ramp_viridis(x)", "ramp_qa_absent(x)")
    )
);

// ── REQ-RENDER-020, R-122: the ramps at the published stops ───────────────────────────────────────────────────────

/// `ramp_viridis` and `ramp_twilight` at their stops `k/(N − 1)` are the published stops, clamp outside [0, 1], and
/// `ramp_grey`, `hue_wheel` are their definitions (OKLab `(t, 0, 0)`; OKLCH at `spec.wheel_lc`, hue `t` turns, in
/// gamut, period 1).
fn check_ramps(gpu: &GpuHarness, spec: Spec) {
    let vir = [0usize, 63, 64, 127, 128, 191, 192, 251, 252, 254, 255];
    let tw = [0usize, 1, 254, 255, 508, 509];
    let mut cases = Vec::new();
    for k in vir {
        cases.push(case(VIRIDIS_OP, &[fb(k as f32 / 255.0)], &[]));
    }
    for k in tw {
        cases.push(case(TWILIGHT_OP, &[fb(k as f32 / 509.0)], &[]));
    }
    let clamps = [-1.0f32, 2.0];
    for t in clamps {
        cases.push(case(VIRIDIS_OP, &[fb(t)], &[]));
        cases.push(case(TWILIGHT_OP, &[fb(t)], &[]));
    }
    let ts: Vec<f32> = (0..=64).map(|j| j as f32 / 64.0).collect();
    for &t in &ts {
        cases.push(case(GREY_OP, &[fb(t)], &[]));
        cases.push(case(WHEEL_OP, &[fb(t)], &[]));
        cases.push(case(WHEEL_OP, &[fb(t + 1.0)], &[]));
        cases.push(case(WHEEL_OP, &[fb(t - 3.0)], &[]));
    }
    let got = eval_rgb(gpu, &cases);
    let mut it = got.into_iter();
    for k in vir {
        let g = it.next().unwrap();
        near(
            &format!("ramp_viridis at stop {k}"),
            g,
            stop(&VIRIDIS, k).map(decode),
        );
    }
    for k in tw {
        let g = it.next().unwrap();
        near(
            &format!("ramp_twilight at stop {k}"),
            g,
            stop(&TWILIGHT, k).map(decode),
        );
    }
    for t in clamps {
        let k = if t < 0.0 { 0 } else { 1 };
        near(
            &format!("ramp_viridis({t})"),
            it.next().unwrap(),
            stop(&VIRIDIS, [0, 255][k]).map(decode),
        );
        near(
            &format!("ramp_twilight({t})"),
            it.next().unwrap(),
            stop(&TWILIGHT, [0, 509][k]).map(decode),
        );
    }
    let (wl, wc) = spec.wheel_lc;
    for &t in &ts {
        let t = f64::from(t);
        near(
            &format!("ramp_grey({t})"),
            it.next().unwrap(),
            oklab_to_linear([t, 0.0, 0.0]),
        );
        for shift in ["", " + 1", " − 3"] {
            let g = it.next().unwrap();
            near(&format!("hue_wheel({t}{shift})"), g, oklch(wl, wc, t));
            assert!(
                g.iter().all(|c| (0.0..=1.0).contains(c)),
                "hue_wheel({t}{shift}) is out of gamut: {g:?}"
            );
        }
    }
}

#[test]
fn qa_prelude_luts_ramps_at_the_published_stops() {
    check_ramps(&gpu(), SPEC);
}

negative_control!(
    qa_prelude_luts_ramps_at_the_published_stops,
    "a hue wheel at another lightness must fail",
    expected = "hue_wheel(0)",
    check_ramps(
        &gpu(),
        Spec {
            wheel_lc: (0.7, 0.12),
            ..SPEC
        }
    )
);

/// The twilight check's own control: a twilight read at viridis's stops must fail.
#[cfg(feature = "controls")]
fn check_twilight_is_not_viridis(gpu: &GpuHarness) {
    let got = eval_rgb(gpu, &[case(TWILIGHT_OP, &[fb(0.0)], &[])]);
    near(
        "ramp_twilight at stop 0",
        got[0],
        stop(&VIRIDIS, 0).map(decode),
    );
}

negative_control!(
    qa_prelude_luts_twilight_is_its_own_table,
    "twilight's first stop is not viridis's",
    expected = "ramp_twilight at stop 0",
    check_twilight_is_not_viridis(&gpu())
);

// ── REQ-RENDER-021: range_norm ─────────────────────────────────────────────────────────────────────────────────────

/// `range_norm(x, lo, hi, auto, meas)` per the requirement, in f64.
fn range_norm_ref(x: f64, lo: f64, hi: f64, auto: bool, meas: [f64; 2], spec: Spec) -> f64 {
    let (l, h) = if auto == spec.auto_selects_meas {
        (meas[0], meas[1])
    } else {
        (lo, hi)
    };
    ((x - l) / (h - l)).clamp(0.0, 1.0)
}

/// The fixtures: x in and out of range, auto off (fixed, clamped) and on (the measured range, here a CPU min/max over
/// a synthetic buffer, as at M1), the two ranges always different so the selection shows. Every range's width is a
/// power of two, so the exact result is representable and the GPU must give it exactly.
fn check_range_norm(gpu: &GpuHarness, spec: Spec) {
    let synthetic = [-3.0f64, 5.0, 1.0, 13.0, 7.5];
    let meas = [
        synthetic.iter().copied().fold(f64::INFINITY, f64::min),
        synthetic.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    ];
    let mut fixtures: Vec<(f64, f64, f64, bool, [f64; 2])> = Vec::new();
    for x in [
        -9.0, -3.0, 0.0, 1.0, 3.0, 4.0, 5.0, 9.0, 13.0, 20.0, 1e30, -1e30,
    ] {
        fixtures.push((x, 0.0, 4.0, false, meas));
        fixtures.push((x, 0.0, 4.0, true, meas));
        fixtures.push((x, -6.0, -2.0, false, [-8.0, 8.0]));
        fixtures.push((x, -6.0, -2.0, true, [-8.0, 8.0]));
    }
    for v in synthetic {
        fixtures.push((v, 100.0, 132.0, true, meas));
    }
    let cases: Vec<Case> = fixtures
        .iter()
        .map(|&(x, lo, hi, auto, m)| {
            case(
                RANGE_NORM,
                &[fb(x as f32), fb(lo as f32), fb(hi as f32)],
                &[u32::from(auto), fb(m[0] as f32), fb(m[1] as f32)],
            )
        })
        .collect();
    for (&(x, lo, hi, auto, m), p) in fixtures.iter().zip(eval(gpu, &cases)) {
        let want = range_norm_ref(x, lo, hi, auto, m, spec);
        let got = f64::from(f32::from_bits(p[0]));
        assert!(
            got == want,
            "range_norm({x}, {lo}, {hi}, auto = {auto}, meas = {m:?}) is {got}, not {want}"
        );
    }
    // The measured extremes map to the ramp's ends under auto.
    let ends = eval(
        gpu,
        &[
            case(
                RANGE_NORM,
                &[fb(meas[0] as f32), fb(0.0), fb(1.0)],
                &[1, fb(meas[0] as f32), fb(meas[1] as f32)],
            ),
            case(
                RANGE_NORM,
                &[fb(meas[1] as f32), fb(0.0), fb(1.0)],
                &[1, fb(meas[0] as f32), fb(meas[1] as f32)],
            ),
        ],
    );
    assert_eq!(
        [f32::from_bits(ends[0][0]), f32::from_bits(ends[1][0])],
        [0.0, 1.0],
        "range_norm under auto does not map the measured min and max to 0 and 1"
    );
}

#[test]
fn qa_range_norm_cpu_reference_vs_shader() {
    check_range_norm(&gpu(), SPEC);
}

negative_control!(
    qa_range_norm_cpu_reference_vs_shader,
    "a reference with auto's selection inverted must fail",
    expected = "range_norm(",
    check_range_norm(
        &gpu(),
        Spec {
            auto_selects_meas: false,
            ..SPEC
        }
    )
);

// ── REQ-TOOL-009, REQ-TOOL-122: the dbg_* helpers ──────────────────────────────────────────────────────────────────

/// The PCG hash as render contract Part 5 defines it (Jarzynski & Olano 2020, `pcg_hash`).
fn pcg(v: u32) -> u32 {
    let state = v.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    let word = ((state >> ((state >> 28) + 4)) ^ state).wrapping_mul(277_803_737);
    (word >> 22) ^ word
}

/// The hatch's colour at `frag_xy`: stripe `((⌊x⌋ + ⌊y⌋) div width) mod 2`.
fn hatch_at(spec: Spec, frag_xy: [f64; 2]) -> [u8; 3] {
    let s = frag_xy[0].floor() as i64 + frag_xy[1].floor() as i64;
    spec.hatch[s.div_euclid(spec.hatch_width).rem_euclid(2) as usize]
}

/// `dbg_cat`: for `n ≤ 8` the Okabe–Ito cycle at `i mod 8`, exactly; for `n > 8` every class on the golden angle,
/// OKLCH at `golden_lc`, hue `frac(i·φ_g)` turns, the same colour whatever `n`, in gamut, and adjacent classes
/// distinct.
fn check_dbg_cat(gpu: &GpuHarness, spec: Spec) {
    let mut cases = Vec::new();
    let small: Vec<(u32, u32)> = (1..=8u32)
        .flat_map(|n| (0..n).map(move |i| (i, n)))
        .chain((8..24u32).map(|i| (i, 8)))
        .collect();
    for &(i, n) in &small {
        cases.push(case(CAT, &[i, n], &[]));
    }
    let classes: Vec<u32> = (0..64u32)
        .chain([255, 256, 1000, 65_535, 1 << 20])
        .collect();
    for &i in &classes {
        for n in [9u32, 64, 1 << 20] {
            cases.push(case(CAT, &[i, n], &[]));
        }
    }
    let got = eval_rgb(gpu, &cases);
    let mut it = got.into_iter();
    for &(i, n) in &small {
        exact8(
            &format!("dbg_cat({i}, {n})"),
            it.next().unwrap(),
            spec.okabe_ito[(i % 8) as usize],
        );
    }
    let (l, c) = spec.golden_lc;
    let mut previous: Option<[i64; 3]> = None;
    for &i in &classes {
        let want = oklch(l, c, (f64::from(i) * spec.phi_g).fract());
        for n in [9u32, 64, 1 << 20] {
            let g = it.next().unwrap();
            near(&format!("dbg_cat({i}, {n})"), g, want);
            assert!(
                g.iter().all(|x| (0.0..=1.0).contains(x)),
                "dbg_cat({i}, {n}) is out of gamut: {g:?}"
            );
            if i < 64 && n == 9 {
                let now = to8(g);
                assert_ne!(
                    Some(now),
                    previous,
                    "dbg_cat classes {i} and its predecessor show one colour"
                );
                previous = Some(now);
            }
        }
    }
}

#[test]
fn qa_dbg_helpers_cat() {
    check_dbg_cat(&gpu(), SPEC);
}

negative_control!(
    qa_dbg_helpers_cat,
    "a golden angle stepping the other way must fail",
    expected = "dbg_cat(1, 9)",
    check_dbg_cat(
        &gpu(),
        Spec {
            phi_g: -SPEC.phi_g + 1.0 - 1e-9,
            ..SPEC
        }
    )
);

negative_control!(
    qa_dbg_helpers_cat_order,
    "an Okabe–Ito palette in another order must fail",
    expected = "dbg_cat(1, 2)",
    check_dbg_cat(
        &gpu(),
        Spec {
            okabe_ito: {
                let mut p = SPEC.okabe_ito;
                p.swap(1, 2);
                p
            },
            ..SPEC
        }
    )
);

negative_control!(
    qa_dbg_helpers_cat_chroma,
    "a golden-angle palette at another chroma must fail",
    expected = "dbg_cat(",
    check_dbg_cat(
        &gpu(),
        Spec {
            golden_lc: (0.75, 0.10),
            ..SPEC
        }
    )
);

/// `dbg_flag`, `dbg_hash_u32`, `dbg_lin` and `dbg_log` return their defined colours on fixture inputs.
fn check_scalar_helpers(gpu: &GpuHarness, spec: Spec) {
    let hashes = [
        0u32,
        1,
        2,
        0xdead_beef,
        0x7fc0_0000,
        0xffff_ffff,
        0x8000_0000,
    ];
    // dbg_lin on [−2, 6]: the ends, beyond them (clamped), a quarter and the middle.
    let lin = [
        (-2.0f32, 0.0f64),
        (6.0, 1.0),
        (-100.0, 0.0),
        (100.0, 1.0),
        (0.0, 0.25),
        (2.0, 0.5),
    ];
    // dbg_log: s = ln(1 + |x|/eps), t = 1 − 1/(1 + s). x = eps·(e^k − 1) gives t = 1 − 1/(1 + k); the sign is not shown,
    // and only |x|/eps matters.
    let e = std::f64::consts::E;
    let mut logs: Vec<(f32, f32, f64)> = vec![(0.0, 1.0, 0.0), (-0.0, 1.0, 0.0)];
    for eps in [1.0f64, 1e-3, 1e3] {
        for k in [1.0f64, 3.0] {
            let x = eps * (e.powf(k) - 1.0);
            let t = 1.0 - 1.0 / (1.0 + k);
            logs.push((x as f32, eps as f32, t));
            logs.push((-x as f32, eps as f32, t));
        }
    }
    logs.push((1e30, 1.0, 1.0 - 1.0 / (1.0 + (1.0f64 + 1e30).ln())));
    let mut cases = vec![case(FLAG, &[1], &[]), case(FLAG, &[0], &[])];
    cases.extend(hashes.iter().map(|&v| case(HASH, &[v], &[])));
    cases.extend(
        lin.iter()
            .map(|&(x, _)| case(LIN, &[fb(x), fb(-2.0), fb(6.0)], &[])),
    );
    cases.extend(
        logs.iter()
            .map(|&(x, eps, _)| case(LOG, &[fb(x), fb(eps)], &[])),
    );
    let got = eval_rgb(gpu, &cases);
    let mut it = got.into_iter();
    exact8("dbg_flag(true)", it.next().unwrap(), spec.flag[0]);
    exact8("dbg_flag(false)", it.next().unwrap(), spec.flag[1]);
    for v in hashes {
        let h = pcg(v).to_le_bytes();
        exact8(
            &format!("dbg_hash_u32({v:#x})"),
            it.next().unwrap(),
            [h[0], h[1], h[2]],
        );
    }
    for (x, t) in lin {
        near(
            &format!("dbg_lin({x}, −2, 6)"),
            it.next().unwrap(),
            viridis(t),
        );
    }
    for (x, eps, t) in logs {
        near(
            &format!("dbg_log({x}, {eps})"),
            it.next().unwrap(),
            viridis(t),
        );
    }
}

#[test]
fn qa_dbg_helpers_flag_hash_lin_log() {
    check_scalar_helpers(&gpu(), SPEC);
}

negative_control!(
    qa_dbg_helpers_flag_hash_lin_log,
    "a flag with green and red swapped must fail",
    expected = "dbg_flag(true)",
    check_scalar_helpers(
        &gpu(),
        Spec {
            flag: [SPEC.flag[1], SPEC.flag[0]],
            ..SPEC
        }
    )
);

/// `dbg_hash_u32` shows change: neighbouring words get different colours.
fn check_hash_changes(gpu: &GpuHarness, step: u32) {
    let vs: Vec<u32> = (0..128u32).map(|k| 0x1234_0000 + k * step).collect();
    let cases: Vec<Case> = vs.iter().map(|&v| case(HASH, &[v], &[])).collect();
    let got: Vec<[i64; 3]> = eval_rgb(gpu, &cases).into_iter().map(to8).collect();
    for w in got.windows(2).zip(vs.windows(2)) {
        assert_ne!(
            w.0[0], w.0[1],
            "dbg_hash_u32 gives {:#x} and {:#x} one colour",
            w.1[0], w.1[1]
        );
    }
}

#[test]
fn qa_dbg_helpers_hash_shows_change() {
    check_hash_changes(&gpu(), 1);
}

negative_control!(
    qa_dbg_helpers_hash_shows_change,
    "an unchanged word must show one colour",
    expected = "one colour",
    check_hash_changes(&gpu(), 0)
);

/// `dbg_sentinel(x, p)`: the absence NaN's exact bits (`0x7FC00000`, lowering Part 3a) draw the hatch at `p`, the same
/// as `debug_invalid(p)`; any finite or infinite value, the stored sentinel −1.0 and the failed state's 0.0 among them,
/// shows its literal value on the viridis ramp at `0.5 + 0.5·x/(1 + |x|)` (render contract Part 5; R-79, R-136), and
/// never a hatch colour.
fn check_sentinel(gpu: &GpuHarness, spec: Spec) {
    let ps: [[f64; 2]; 6] = [
        [0.5, 0.5],
        [4.5, 0.5],
        [2.5, 2.5],
        [7.5, 0.5],
        [17.25, 30.75],
        [100.5, 3.5],
    ];
    let values = [
        -1.0f32,
        0.0,
        -0.0,
        1.0,
        1e-45,
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];
    let mut cases = Vec::new();
    for p in ps {
        let pb = [fb(p[0] as f32), fb(p[1] as f32)];
        cases.push(case(SENTINEL, &[0x7fc0_0000], &pb));
        cases.push(case(INVALID, &[], &pb));
        for v in values {
            cases.push(case(SENTINEL, &[fb(v)], &pb));
        }
    }
    let got = eval_rgb(gpu, &cases);
    let mut it = got.into_iter();
    for p in ps {
        let nan = it.next().unwrap();
        let invalid = it.next().unwrap();
        exact8(&format!("dbg_sentinel(NaN, {p:?})"), nan, hatch_at(spec, p));
        assert_eq!(
            to8(nan),
            to8(invalid),
            "dbg_sentinel(NaN, {p:?}) is not debug_invalid({p:?})"
        );
        for v in values {
            let g = it.next().unwrap();
            let x = f64::from(v).clamp(-1e30, 1e30);
            near(
                &format!("dbg_sentinel({v}, {p:?})"),
                g,
                viridis(0.5 + 0.5 * x / (1.0 + x.abs())),
            );
            for h in spec.hatch {
                assert_ne!(
                    to8(g),
                    h.map(i64::from),
                    "dbg_sentinel({v}) draws a hatch colour (only NaN gets the hatch, R-136)"
                );
            }
        }
    }
    // The fixture the requirement names: −1.0 at a quarter of the ramp (stops 63 and 64), not the hatch.
    let g = eval_rgb(gpu, &[case(SENTINEL, &[fb(-1.0)], &[fb(0.5), fb(0.5)])])[0];
    near("dbg_sentinel(−1.0)", g, viridis(0.25));
}

#[test]
fn qa_dbg_helpers_sentinel() {
    check_sentinel(&gpu(), SPEC);
}

negative_control!(
    qa_dbg_helpers_sentinel,
    "a hatch with its stripes swapped must fail",
    expected = "dbg_sentinel(NaN, [0.5, 0.5])",
    check_sentinel(
        &gpu(),
        Spec {
            hatch: [SPEC.hatch[1], SPEC.hatch[0]],
            ..SPEC
        }
    )
);

/// The rendered field view: `dbg_sentinel` over a whole target, every pixel reading `bits`. The absence NaN draws the
/// hatch from each pixel's own position, both colours present; any other value is one flat colour.
fn check_sentinel_image(gpu: &GpuHarness, bits: u32, spec: Spec) {
    let (w, h) = (32u32, 16u32);
    let px = draw(gpu, "qa_sentinel", &[bits, 0, 0, 0], w, h);
    let mut seen = std::collections::BTreeSet::new();
    for (k, p) in px.iter().enumerate() {
        let xy = [(k as u32 % w) as f64 + 0.5, (k as u32 / w) as f64 + 0.5];
        exact8(
            &format!("the sentinel view at {xy:?}"),
            rgb(*p),
            hatch_at(spec, xy),
        );
        seen.insert(to8(rgb(*p)));
    }
    assert_eq!(seen.len(), 2, "the hatch is not two colours: {seen:?}");
}

#[test]
fn qa_dbg_helpers_sentinel_view_hatches_nan() {
    check_sentinel_image(&gpu(), 0x7fc0_0000, SPEC);
}

negative_control!(
    qa_dbg_helpers_sentinel_view_hatches_nan,
    "a view of the stored sentinel −1.0 must not show the hatch",
    expected = "the sentinel view at",
    check_sentinel_image(&gpu(), (-1.0f32).to_bits(), SPEC)
);

// ── REQ-COL-055: the hatch, and no collision ───────────────────────────────────────────────────────────────────────

/// `debug_invalid(frag_xy)` over a whole target draws, at each pixel's own position, the proposal's stripes: `width`
/// px wide across `x + y`, the two proposed colours exactly.
fn check_hatch(gpu: &GpuHarness, spec: Spec) {
    let (w, h) = (64u32, 64u32);
    let px = draw(gpu, "qa_hatch", &[0, 0, 0, 0], w, h);
    for (k, p) in px.iter().enumerate() {
        let xy = [(k as u32 % w) as f64 + 0.5, (k as u32 / w) as f64 + 0.5];
        exact8(
            &format!("debug_invalid at {xy:?}"),
            rgb(*p),
            hatch_at(spec, xy),
        );
    }
}

#[test]
fn qa_dbg_helpers_hatch_pattern() {
    check_hatch(&gpu(), SPEC);
}

negative_control!(
    qa_dbg_helpers_hatch_pattern,
    "stripes 2 px wide must fail",
    expected = "debug_invalid at",
    check_hatch(
        &gpu(),
        Spec {
            hatch_width: 2,
            ..SPEC
        }
    )
);

/// A published table: its name, its stop count and qa's anchor stops.
type Table = (&'static str, usize, &'static [(usize, Rgb)]);

/// colour_composition §7.1's LUTs whose stops are a published table, by name: the table file under
/// `crates/render/tests/data/lut/` (sRGB-encoded stops in [0, 1], one per line, `#` comments), its stop count, and
/// stops qa transcribed independently from the source the file names, matplotlib 3.8.0's `_cm_listed.py`
/// (`_<name>_data`) and `_cm.py` (`_coolwarm_data`, Moreland's table), so that the file is checked against the table
/// before it is measured. Viridis and twilight are drawn by the prelude's own ramps on the GPU ([`palette`]).
const TABLES: [Table; 6] = [
    (
        "cividis",
        256,
        &[
            (0, [0.0, 0.135112, 0.304751]),
            (255, [0.995737, 0.909344, 0.217772]),
        ],
    ),
    (
        "plasma",
        256,
        &[
            (0, [0.050383, 0.029803, 0.527975]),
            (71, [0.534952, 0.031217, 0.650165]),
            (72, [0.54057, 0.03495, 0.64864]),
            (255, [0.940015, 0.975158, 0.131326]),
        ],
    ),
    (
        "magma",
        256,
        &[
            (0, [0.001462, 0.000466, 0.013866]),
            (255, [0.987053, 0.991438, 0.749504]),
        ],
    ),
    (
        "inferno",
        256,
        &[
            (0, [0.001462, 0.000466, 0.013866]),
            (255, [0.988362, 0.998364, 0.644924]),
        ],
    ),
    (
        "turbo",
        256,
        &[
            (0, [0.18995, 0.07176, 0.23217]),
            (83, [0.09662, 0.88454, 0.73316]),
            (84, [0.09958, 0.8904, 0.72393]),
            (255, [0.4796, 0.01583, 0.01055]),
        ],
    ),
    (
        "coolwarm",
        33,
        &[
            (0, [0.2298057, 0.298717966, 0.753683153]),
            (16, [0.865395197, 0.86541021, 0.865395561]),
            (32, [0.705673158, 0.01555616, 0.150232812]),
        ],
    ),
];

/// A published table's stops, read from its file and checked against `anchors` and `count`.
fn table(name: &str, count: usize, anchors: &[(usize, Rgb)]) -> Vec<Rgb> {
    let text = read(&format!("crates/render/tests/data/lut/{name}.txt"));
    let stops: Vec<Rgb> = text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| {
            let v: Vec<f64> = l.split_whitespace().map(|x| x.parse().unwrap()).collect();
            assert_eq!(v.len(), 3, "{name}: the line {l:?} is not one stop");
            [v[0], v[1], v[2]]
        })
        .collect();
    assert_eq!(
        stops.len(),
        count,
        "{name}: the table has {} stops, the source {count}",
        stops.len()
    );
    for (k, want) in anchors {
        assert_eq!(stops[*k], *want, "{name}: stop {k} is not the source's");
    }
    stops
}

/// The Principia palette's eight stops, read from the oracle itself, `principia_colour_explorer.html`'s
/// `LUT.principia` (colour_composition §7.1; R-122), 8-bit sRGB as encoded fractions.
fn principia_stops() -> Vec<Rgb> {
    let html = read("docs/gui/reference/principia_colour_explorer.html");
    let at = html
        .find("principia:[[")
        .expect("the explorer has no LUT.principia")
        + "principia:[".len();
    let body = &html[at..at + html[at..].find("]]").unwrap() + 1];
    let stops: Vec<Rgb> = body
        .split("],")
        .map(|s| {
            let v: Vec<f64> = s
                .trim_matches(|c| c == '[' || c == ']')
                .split(',')
                .map(|x| x.trim().parse::<f64>().unwrap() / 255.0)
                .collect();
            [v[0], v[1], v[2]]
        })
        .collect();
    assert_eq!(
        stops.len(),
        8,
        "LUT.principia has {} stops, not eight",
        stops.len()
    );
    stops
}

/// Cubehelix's reference, the analytic form with dd_colouring §3.8's parameters (s = 0.5, λ = 1.5, h = 1; R-151),
/// clamped to [0, 1] and read as sRGB-encoded, as the explorer draws it.
fn cubehelix(t: f64) -> Rgb {
    let phi = std::f64::consts::TAU * (0.5 / 3.0 - 1.5 * t);
    let a = t * (1.0 - t) / 2.0;
    let (c, s) = (phi.cos(), phi.sin());
    [
        t + a * (-0.14861 * c + 1.78277 * s),
        t + a * (-0.29227 * c - 0.90649 * s),
        t + a * (1.97294 * c),
    ]
    .map(|v| v.clamp(0.0, 1.0))
}

/// Samples per interval between two stops.
const PER_INTERVAL: usize = 32;

/// Every LUT of colour_composition §7.1 that the GPU ramps don't draw, as linear RGB: cividis, plasma, magma, inferno,
/// Turbo, Cool-warm and the Principia palette at every stop and between each pair of neighbouring stops, mixed both in
/// the sRGB encoding and in linear light (either way a ramp may draw them), and cubehelix along its analytic curve.
/// Each entry is named `<lut> …` so a collision names its LUT.
fn lut_palette() -> Vec<(String, Rgb)> {
    let mut luts: Vec<(&str, Vec<Rgb>)> = TABLES
        .iter()
        .map(|(n, c, a)| (*n, table(n, *c, a)))
        .collect();
    luts.push(("principia", principia_stops()));
    let mut out = Vec::new();
    for (name, stops) in &luts {
        for (k, w) in stops.windows(2).enumerate() {
            let (a, b) = (w[0], w[1]);
            let (la, lb) = (a.map(decode), b.map(decode));
            for j in 0..=PER_INTERVAL {
                let f = j as f64 / PER_INTERVAL as f64;
                let enc = [0, 1, 2].map(|c| decode(a[c] + (b[c] - a[c]) * f));
                let lin = [0, 1, 2].map(|c| la[c] + (lb[c] - la[c]) * f);
                out.push((format!("{name} stop {k} + {f:.3} (sRGB mix)"), enc));
                out.push((format!("{name} stop {k} + {f:.3} (linear mix)"), lin));
            }
        }
    }
    let n = 255 * PER_INTERVAL;
    for j in 0..=n {
        let t = j as f64 / n as f64;
        out.push((format!("cubehelix at {t:.5}"), cubehelix(t).map(decode)));
    }
    out
}

/// One colour on each §7.1 LUT, as [`lut_palette`] draws it (mid-interval, in the sRGB mix), by the LUT's name.
fn on_each_lut() -> Vec<(&'static str, Rgb)> {
    let mut v: Vec<(&'static str, Rgb)> = TABLES
        .iter()
        .map(|(n, c, a)| {
            let s = table(n, *c, a);
            let k = c / 3;
            (*n, [0, 1, 2].map(|i| decode((s[k][i] + s[k + 1][i]) / 2.0)))
        })
        .collect();
    let p = principia_stops();
    v.push((
        "principia",
        [0, 1, 2].map(|i| decode((p[3][i] + p[4][i]) / 2.0)),
    ));
    v.push(("cubehelix", cubehelix(0.37).map(decode)));
    v
}

/// Every palette entry the requirement names: as the GPU draws them, `dbg_cat`'s Okabe–Ito and golden-angle classes,
/// `dbg_flag`, viridis, twilight, the grey ramp and the hue wheel, sampled; and the outcome palette with `#E034C6` and
/// every LUT of colour_composition §7.1 ([`lut_palette`]).
fn palette(gpu: &GpuHarness) -> Vec<(String, Rgb)> {
    let mut named = Vec::new();
    let mut cases = Vec::new();
    for i in 0..8u32 {
        named.push(format!("dbg_cat({i}, 8)"));
        cases.push(case(CAT, &[i, 8], &[]));
    }
    for i in 0..512u32 {
        named.push(format!("dbg_cat({i}, 512)"));
        cases.push(case(CAT, &[i, 512], &[]));
    }
    for b in 0..2u32 {
        named.push(format!("dbg_flag({b})"));
        cases.push(case(FLAG, &[b], &[]));
    }
    for j in 0..=2048u32 {
        let t = j as f32 / 2048.0;
        for (op, name) in [
            (VIRIDIS_OP, "ramp_viridis"),
            (TWILIGHT_OP, "ramp_twilight"),
            (GREY_OP, "ramp_grey"),
            (WHEEL_OP, "hue_wheel"),
        ] {
            named.push(format!("{name}({t})"));
            cases.push(case(op, &[fb(t)], &[]));
        }
    }
    let mut out: Vec<(String, Rgb)> = named.into_iter().zip(eval_rgb(gpu, &cases)).collect();
    out.extend(
        OUTCOME
            .iter()
            .map(|&h| (format!("outcome #{h:06X}"), hex(h))),
    );
    out.extend(lut_palette());
    out
}

/// Each of `hatch`'s colours is farther in OKLab from every palette entry than the flat magenta `#FF00FF` is from
/// `#E034C6`, the collision R-132 ruled out.
fn check_no_collision(hatch: [Rgb; 2], palette: &[(String, Rgb)]) {
    let ruled = dist(hex(0xFF00FF), hex(0xE034C6));
    for (k, c) in hatch.iter().enumerate() {
        let (d, name) = palette
            .iter()
            .map(|(n, p)| (dist(*c, *p), n))
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .unwrap();
        assert!(
            d > ruled,
            "the hatch's colour {k} collides with {name}: {d:.4} in OKLab, no farther than #FF00FF from #E034C6 \
             ({ruled:.4})"
        );
    }
}

#[test]
fn qa_dbg_helpers_hatch_collides_with_no_palette_entry() {
    let gpu = gpu();
    let drawn = eval_rgb(
        &gpu,
        &[
            case(INVALID, &[], &[fb(0.5), fb(0.5)]),
            case(INVALID, &[], &[fb(4.5), fb(0.5)]),
        ],
    );
    check_no_collision([drawn[0], drawn[1]], &palette(&gpu));
}

negative_control!(
    qa_dbg_helpers_hatch_collides_with_no_palette_entry,
    "the flat magenta R-16 kept must collide",
    expected = "collides with outcome #E034C6",
    check_no_collision([hex(0xFF00FF), hex(0x48FFFF)], &palette(&gpu()))
);

/// Each hatch colour is clear of every §7.1 LUT on its own, each LUT measured: the palette has entries named for all
/// ten (viridis and twilight from the GPU ramps, the rest from [`lut_palette`]), and a colour taken on each LUT is
/// found to collide with that LUT by name, so no LUT is missing from the measurement or mislabelled.
fn check_every_lut_measured(hatch: [Rgb; 2], palette: &[(String, Rgb)]) {
    for name in [
        "ramp_viridis",
        "cividis",
        "plasma",
        "magma",
        "inferno",
        "ramp_twilight",
        "coolwarm",
        "principia",
        "cubehelix",
        "turbo",
    ] {
        let n = palette.iter().filter(|(p, _)| p.starts_with(name)).count();
        assert!(n >= 33, "the palette samples {name} {n} times");
    }
    for (name, c) in on_each_lut() {
        let got = std::panic::catch_unwind(|| check_no_collision([c, c], palette));
        let msg = match got {
            Ok(()) => panic!("a colour on {name} is not found to collide with it"),
            Err(e) => e.downcast_ref::<String>().cloned().unwrap_or_default(),
        };
        assert!(
            msg.contains(&format!("collides with {name}")),
            "a colour on {name} is matched elsewhere: {msg}"
        );
    }
    check_no_collision(hatch, palette);
}

#[test]
fn qa_hatch_clear_of_every_colour_composition_lut() {
    let gpu = gpu();
    let drawn = eval_rgb(
        &gpu,
        &[
            case(INVALID, &[], &[fb(0.5), fb(0.5)]),
            case(INVALID, &[], &[fb(4.5), fb(0.5)]),
        ],
    );
    check_every_lut_measured([drawn[0], drawn[1]], &palette(&gpu));
}

negative_control!(
    qa_hatch_clear_of_every_colour_composition_lut,
    "the aquamarine #50FFD2 physics review 5406355539 found 0.072 from Turbo must collide with turbo",
    expected = "collides with turbo",
    check_every_lut_measured([hex(0x9B00FF), hex(0x50FFD2)], &palette(&gpu()))
);
