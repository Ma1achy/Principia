//! The combiners and `None` as the identity of `combine` (TASK-M7-04; dd_colouring §3.5, §5 unit test 6;
//! colour_composition §4.1; render contract Part 4; R-77):
//! - REQ-COL-038: Replace-L leaves `(a, b_ab)` bit-stable; at the default `L_min = 0`, `L_max = 1` its L is `b`
//!   exactly; a non-default range maps `b = 0` and `b = 1` to `L_min` and `L_max` exactly; Multiply is `rgb·b` in
//!   linear space and preserves channel ratios. Each on the Rust mirror (`render::colour::combine`) and on the WGSL
//!   occupants on the GPU (`combiner_replace_l_*`);
//! - REQ-COL-009: under Replace-L the output's OKLab L is the brightness's, whatever the colour, and the colour gives
//!   the hue and chroma (`combiner_l_ownership_*`);
//! - REQ-COL-023: each of the four colour/brightness combinations, under Replace-L and Multiply, gives the truth
//!   table's result: on the mirror, in the generated `shade()`'s text, and drawn through the assembled stain on the
//!   GPU (`combiner_none_identity_*`).
//!
//! Every check takes its subject as an argument, and its negative control runs it on a wrong one (R-176).

use ledger::gen::prelude;
use render::assemble::{self, Kind, Node, Occupant, Stain, Tier};
use render::codegen;
use render::colour::combine::{self, Combiner, LRange, MID_GREY_L, WHITE};
use render::colour::space;
use validation::gpu::{BindingKind, GpuHarness};
use validation::negative_control;

const COLOUR_SPACE: &str = include_str!("../shaders/wgsl/lib/colour_space.wgsl");
const REPLACE_L: &str = include_str!("../shaders/wgsl/frag/combiner/replace_l.wgsl");
const MULTIPLY: &str = include_str!("../shaders/wgsl/frag/combiner/multiply.wgsl");

// ── The cases ────────────────────────────────────────────────────────────────────────────────────────────────────

/// The colours: every sRGB code of a 6-level lattice (0, 51, … 255 per channel), decoded to linear sRGB, so black,
/// white, the greys, the primaries and secondaries and the saturated edges of the cube.
fn colours() -> Vec<[f64; 3]> {
    let levels: Vec<f64> = (0..6)
        .map(|k| space::srgb_to_linear(f64::from(k * 51) / 255.0))
        .collect();
    let mut out = Vec::new();
    for &r in &levels {
        for &g in &levels {
            for &b in &levels {
                out.push([r, g, b]);
            }
        }
    }
    out
}

/// The brightnesses: both ends, values exact in binary and values that are not.
const BRIGHTNESSES: [f64; 10] = [0.0, 0.1, 0.25, 1.0 / 3.0, 0.5, 0.6, 0.75, 0.9, 0.999, 1.0];

/// Non-default lightness ranges.
const RANGES: [LRange; 4] = [
    LRange {
        l_min: 0.2,
        l_max: 0.8,
    },
    LRange {
        l_min: 0.1,
        l_max: 0.7,
    },
    LRange {
        l_min: 0.05,
        l_max: 0.95,
    },
    LRange {
        l_min: 0.35,
        l_max: 0.9,
    },
];

fn f32s(x: [f64; 3]) -> [f32; 3] {
    x.map(|c| c as f32)
}

fn wide(x: [f32; 3]) -> [f64; 3] {
    x.map(f64::from)
}

/// Replace-L's OKLab step, as a subject: `(range, lab, b) → lab`.
type Step<T> = fn(LRange, [T; 3], T) -> [T; 3];

// ── REQ-COL-038 on the mirror ────────────────────────────────────────────────────────────────────────────────────

/// `step` leaves `(a, b_ab)` bit for bit, in f32 and f64, at the default range and the others, for every colour's
/// OKLab and every brightness.
fn check_ab_bit_stable(step32: Step<f32>, step64: Step<f64>) {
    for range in [LRange::DEFAULT].iter().chain(&RANGES) {
        for rgb in colours() {
            let lab = space::linear_to_oklab(rgb);
            let lab32 = space::linear_to_oklab(f32s(rgb));
            for b in BRIGHTNESSES {
                let got = step64(*range, lab, b);
                assert!(
                    got[1].to_bits() == lab[1].to_bits() && got[2].to_bits() == lab[2].to_bits(),
                    "f64: (a, b_ab) not bit-stable: {lab:?} → {got:?} at b = {b}, {range:?}"
                );
                let got = step32(*range, lab32, b as f32);
                assert!(
                    got[1].to_bits() == lab32[1].to_bits()
                        && got[2].to_bits() == lab32[2].to_bits(),
                    "f32: (a, b_ab) not bit-stable: {lab32:?} → {got:?} at b = {b}, {range:?}"
                );
            }
        }
    }
}

#[test]
fn combiner_replace_l_keeps_a_and_b_bit_stable() {
    check_ab_bit_stable(combine::replace_l_lab, combine::replace_l_lab);
}

negative_control!(
    combiner_replace_l_keeps_a_and_b_bit_stable,
    "a step that rounds `a` must not be bit-stable",
    expected = "not bit-stable",
    check_ab_bit_stable(
        |range, lab, b| {
            let [l, ..] = combine::replace_l_lab(range, lab, b);
            [l, (f64::from(lab[1]) * (1.0 + 1e-7)) as f32, lab[2]]
        },
        |range, lab, b| {
            let [l, ..] = combine::replace_l_lab(range, lab, b);
            [l, f64::from(lab[1] as f32), lab[2]]
        }
    )
);

/// `range`'s lightness for every brightness, f32 and f64, against `want(b)`: exactly, bit for bit.
fn check_lightness(range: LRange, want: fn(LRange, f64) -> Option<f64>) {
    for b in BRIGHTNESSES {
        if let Some(w) = want(range, b) {
            let got = combine::replace_l_lab(range, [0.5, 0.1, -0.1], b)[0];
            assert!(
                got.to_bits() == w.to_bits(),
                "f64: L = {got} at b = {b}, {range:?}; want exactly {w}"
            );
            let got = combine::replace_l_lab(range, [0.5f32, 0.1, -0.1], b as f32)[0];
            assert!(
                got.to_bits() == (w as f32).to_bits(),
                "f32: L = {got} at b = {b}, {range:?}; want exactly {}",
                w as f32
            );
        }
    }
}

/// At the defaults, L is b.
fn l_is_b(_: LRange, b: f64) -> Option<f64> {
    Some(b)
}

/// The defaults are R-77's, and at them L is b, exactly, in the mirror and in the WGSL occupant's declared defaults.
fn check_default_l_is_b(range: LRange) {
    assert!(
        range.l_min == 0.0 && range.l_max == 1.0,
        "the default range is {range:?}, not L_min = 0, L_max = 1 (R-77)"
    );
    check_lightness(range, l_is_b);
}

#[test]
fn combiner_replace_l_default_l_is_b() {
    check_default_l_is_b(LRange::DEFAULT);
    check_default_l_is_b(LRange::default());
    let d = assemble::declaration(Kind::Combiner, &Occupant::BuiltIn("replace_l".into()))
        .unwrap_or_else(|e| panic!("{e}"));
    let declared = |name: &str| {
        d.uniforms
            .iter()
            .find(|u| u.name == name)
            .map(|u| u.default.clone())
            .unwrap_or_else(|| panic!("replace_l.wgsl declares no `{name}`"))
    };
    check_default_l_is_b(LRange {
        l_min: declared("l_min")[0],
        l_max: declared("l_max")[0],
    });
}

negative_control!(
    combiner_replace_l_default_l_is_b,
    "a default of L_max = 0.95 must miss L = b",
    expected = "not L_min = 0, L_max = 1",
    check_default_l_is_b(LRange {
        l_min: 0.0,
        l_max: 0.95
    })
);

/// `b = 0` gives L_min and `b = 1` gives L_max, exactly; between them L is `L_min + (L_max − L_min)·b` within the
/// rounding of its evaluation.
fn ends(range: LRange, b: f64) -> Option<f64> {
    match b {
        0.0 => Some(range.l_min),
        1.0 => Some(range.l_max),
        _ => None,
    }
}

fn check_range_ends(ranges: &[LRange], lightness: fn(LRange, f64) -> f64) {
    for &range in ranges {
        check_lightness(range, ends);
        for b in BRIGHTNESSES {
            let want = range.l_min + (range.l_max - range.l_min) * b;
            let got = lightness(range, b);
            assert!(
                (got - want).abs() <= 4.0 * f64::EPSILON,
                "L = {got} at b = {b}, {range:?}; the range form gives {want}"
            );
        }
    }
}

fn mirror_lightness(range: LRange, b: f64) -> f64 {
    combine::replace_l_lab(range, [0.5, 0.0, 0.0], b)[0]
}

#[test]
fn combiner_replace_l_range_maps_the_ends() {
    check_range_ends(&RANGES, mirror_lightness);
}

negative_control!(
    combiner_replace_l_range_maps_the_ends,
    "a lightness that ignores the range, L = b, must miss L_min + (L_max − L_min)·b",
    expected = "the range form gives",
    check_range_ends(&RANGES, |_, b| b)
);

/// `multiply` keeps each pair of channels' ratio, within the products' rounding, keeps a zero channel zero, and is
/// `rgb·b` channel by channel: f64 within its rounding, f32 bit for bit.
fn check_ratios(multiply64: fn([f64; 3], f64) -> [f64; 3]) {
    for rgb in colours() {
        for b in BRIGHTNESSES.into_iter().filter(|&b| b > 0.0) {
            let out = multiply64(rgb, b);
            for i in 0..3 {
                for j in 0..3 {
                    if rgb[j] > 0.0 {
                        let (want, got) = (rgb[i] / rgb[j], out[i] / out[j]);
                        assert!(
                            (got - want).abs() <= 4.0 * f64::EPSILON * want,
                            "channel ratio {i}/{j} of {rgb:?} is {want}, after ·{b} {got}"
                        );
                    } else {
                        assert!(
                            out[j] == 0.0,
                            "channel {j} of {rgb:?} is 0, after ·{b} {}",
                            out[j]
                        );
                    }
                }
            }
        }
    }
    for rgb in colours() {
        for b in BRIGHTNESSES.into_iter().filter(|&b| b > 0.0) {
            let out = multiply64(rgb, b);
            for i in 0..3 {
                assert!(
                    (out[i] - rgb[i] * b).abs() <= f64::EPSILON * rgb[i] * b,
                    "{rgb:?}·{b} = {out:?} is not rgb·b in linear space"
                );
            }
            let out32 = combine::multiply(f32s(rgb), b as f32);
            let want32 = f32s(rgb).map(|c| c * b as f32);
            assert!(
                out32.map(f32::to_bits) == want32.map(f32::to_bits),
                "f32: {rgb:?}·{b} = {out32:?}, want {want32:?}"
            );
        }
    }
}

#[test]
fn combiner_replace_l_multiply_preserves_channel_ratios() {
    check_ratios(combine::multiply);
}

negative_control!(
    combiner_replace_l_multiply_preserves_channel_ratios,
    "Multiply in encoded sRGB rather than linear space must break the channel ratios",
    expected = "channel ratio",
    check_ratios(|rgb, b| rgb.map(|c| space::srgb_to_linear(space::linear_to_srgb(c) * b)))
);

// ── REQ-COL-038 on the GPU: the WGSL occupants ───────────────────────────────────────────────────────────────────

/// The test entry appended to the prelude, `colour_space.wgsl` and one combiner file: pixel `i` takes case `i` (the
/// padding pixels repeat the last), eight words: the operation, a colour's three f32 words, `b`, and `L`. Operation 1
/// is Replace-L's OKLab step on the colour as OKLab; 2 the file's `combine`; 3 the table's composition written here:
/// linear sRGB → OKLab, L ← `L`, → linear sRGB, through the shared library. The combiner's uniforms are a struct at
/// group 1, as the assembler gives a node's.
const ENTRY: &str = r"
struct CombinerUniforms {
    l_min: f32,
    l_max: f32,
}
@group(1) @binding(0) var<uniform> uniforms: CombinerUniforms;
@group(2) @binding(0) var<storage, read> cases: array<u32>;

const CASE_WIDTH: u32 = 256u;

@fragment
fn t_combiner(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<u32> {
    let i = min(u32(pos.y) * CASE_WIDTH + u32(pos.x), arrayLength(&cases) / 8u - 1u) * 8u;
    let x = bitcast<vec3<f32>>(vec3<u32>(cases[i + 1u], cases[i + 2u], cases[i + 3u]));
    let b = bitcast<f32>(cases[i + 4u]);
    let l = bitcast<f32>(cases[i + 5u]);
    var r = vec3<f32>(0.0);
    switch cases[i] {
        case 1u: { r = OPERATION_STEP; }
        case 2u: { r = combine(x, b); }
        case 3u: { r = oklab_to_linear(vec3<f32>(l, linear_to_oklab(x).yz)); }
        default: {}
    }
    return vec4<u32>(bitcast<vec3<u32>>(r), 0u);
}
";

const CASE_WIDTH: usize = 256;
const STEP: u32 = 1;
const COMBINE: u32 = 2;
const COMPOSITION: u32 = 3;

/// A combiner file under test, and what the entry's operation 1 calls.
#[derive(Clone, Copy)]
struct File<'a> {
    text: &'a str,
    step: &'a str,
}

const REPLACE_L_FILE: File<'static> = File {
    text: REPLACE_L,
    step: "replace_l_lab(x, b)",
};

const MULTIPLY_FILE: File<'static> = File {
    text: MULTIPLY,
    step: "x",
};

/// Each `(op, colour, b, L)` evaluated on the GPU after the prelude, `colour_space.wgsl` and `file`, the combiner's
/// uniforms `range`: one draw, a pixel per case.
fn run(file: File, range: LRange, cases: &[(u32, [f32; 3], f32, f32)]) -> Vec<[f32; 3]> {
    let gpu = GpuHarness::new().expect("a GPU device");
    let module = format!(
        "{}\n{COLOUR_SPACE}\n{}\n{}",
        prelude::wgsl(Tier::FULL),
        file.text,
        ENTRY.replace("OPERATION_STEP", file.step)
    );
    let words: Vec<u32> = cases
        .iter()
        .flat_map(|&(op, x, b, l)| {
            [
                op,
                x[0].to_bits(),
                x[1].to_bits(),
                x[2].to_bits(),
                b.to_bits(),
                l.to_bits(),
                0,
                0,
            ]
        })
        .collect();
    let uniforms = [
        (range.l_min as f32).to_bits(),
        (range.l_max as f32).to_bits(),
        0,
        0,
    ];
    let width = cases.len().min(CASE_WIDTH);
    let rows = cases.len().div_ceil(CASE_WIDTH);
    let kernel = gpu
        .fragment(
            &module,
            "t_combiner",
            &[&[], &[BindingKind::Uniform], &[BindingKind::Storage]],
            width as u32,
            rows as u32,
        )
        .unwrap_or_else(|e| panic!("{e}"));
    let pixels = kernel
        .draw(&[&[], &[&uniforms], &[&words]])
        .unwrap_or_else(|e| panic!("{e}"));
    pixels[..cases.len()]
        .iter()
        .map(|p| [p[0], p[1], p[2]].map(f32::from_bits))
        .collect()
}

/// Every colour's f32 OKLab (the mirror's) with every brightness, as `op` cases.
fn lab_cases(op: u32) -> Vec<(u32, [f32; 3], f32, f32)> {
    let mut out = Vec::new();
    for rgb in colours() {
        let lab = space::linear_to_oklab(f32s(rgb));
        for b in BRIGHTNESSES {
            out.push((op, lab, b as f32, 0.0));
        }
    }
    out
}

/// The WGSL step, `file`'s, at the default range and each other: `(a, b_ab)` bit for bit as given; L the mirror's f32
/// lightness bit for bit at the defaults (so `b`) and at the ends (so L_min, L_max). Between the ends of another range
/// WGSL lets the compiler fuse `L_min·(1 − b) + L_max·b` into a fused multiply-add, which skips one product's rounding:
/// the two evaluations share `1 − b`'s rounding and differ by at most the product's and the sum's, each within
/// `ε·(|L_min| + |L_max|)`, so `2ε·(|L_min| + |L_max|)`.
fn check_gpu_step(file: File) {
    let cases = lab_cases(STEP);
    for range in [LRange::DEFAULT].iter().chain(&RANGES) {
        let got = run(file, *range, &cases);
        for ((_, lab, b, _), g) in cases.iter().zip(&got) {
            assert!(
                g[1].to_bits() == lab[1].to_bits() && g[2].to_bits() == lab[2].to_bits(),
                "GPU: (a, b_ab) not bit-stable: {lab:?} → {g:?} at b = {b}, {range:?}"
            );
            let want = range.lightness(*b);
            let exact = *range == LRange::DEFAULT || *b == 0.0 || *b == 1.0;
            let bound = if exact {
                0.0
            } else {
                2.0 * f64::from(f32::EPSILON) * (range.l_min.abs() + range.l_max.abs())
            };
            assert!(
                (f64::from(g[0]) - f64::from(want)).abs() <= bound,
                "GPU: L = {} at b = {b}, {range:?}; the mirror's is {want}, within {bound}",
                g[0]
            );
        }
    }
}

#[test]
fn combiner_replace_l_wgsl_step() {
    check_gpu_step(REPLACE_L_FILE);
}

negative_control!(
    combiner_replace_l_wgsl_step,
    "a WGSL step that recomputes `a` through the polar form must not be bit-stable",
    expected = "not bit-stable",
    check_gpu_step(File {
        text: REPLACE_L,
        step: "oklch_to_oklab(oklab_to_oklch(replace_l_lab(x, b)))",
    })
);

/// The WGSL `combine`, `file`'s, at `range`, on every colour and brightness, against `compare`: a value outright, bit
/// for bit; or the composition the entry writes through the shared library, linear sRGB → OKLab, L ← the lightness the
/// GPU's own step gives for `b` (checked against the mirror by `combiner_replace_l_wgsl_step`), → linear sRGB, within
/// [`TOL_F32`] in OKLab ([`near_in_oklab`]: the compiler may order the two evaluations differently).
fn check_gpu_combine(file: File, range: LRange, compare: fn([f32; 3], f32) -> Expected) {
    let mut cases = Vec::new();
    for rgb in colours() {
        for b in BRIGHTNESSES {
            cases.push((COMBINE, f32s(rgb), b as f32, 0.0));
        }
    }
    let got = run(file, range, &cases);
    let expected: Vec<Expected> = cases.iter().map(|&(_, x, b, _)| compare(x, b)).collect();
    let composed = if expected.iter().any(|e| matches!(e, Expected::Composition)) {
        let steps: Vec<_> = cases
            .iter()
            .map(|&(_, _, b, _)| (STEP, [0.0; 3], b, 0.0))
            .collect();
        let lightness = run(file, range, &steps);
        let reference: Vec<_> = cases
            .iter()
            .zip(&lightness)
            .map(|(&(_, x, b, _), l)| (COMPOSITION, x, b, l[0]))
            .collect();
        run(file, range, &reference)
    } else {
        Vec::new()
    };
    for (k, ((_, x, b, _), g)) in cases.iter().zip(&got).enumerate() {
        let ok = match expected[k] {
            Expected::Composition => near_in_oklab(*g, composed[k]),
            Expected::Value(v) => g.map(f32::to_bits) == v.map(f32::to_bits),
        };
        assert!(
            ok,
            "GPU combine({x:?}, {b}) = {g:?}; want {:?} ({range:?})",
            match expected[k] {
                Expected::Composition => composed[k],
                Expected::Value(v) => v,
            }
        );
    }
}

/// What the GPU's `combine` must give: the composition, or a value.
enum Expected {
    Composition,
    Value([f32; 3]),
}

fn replace_l_expected(_: [f32; 3], _: f32) -> Expected {
    Expected::Composition
}

fn multiply_expected(x: [f32; 3], b: f32) -> Expected {
    Expected::Value(combine::multiply(x, b))
}

#[test]
fn combiner_replace_l_wgsl_converts_sets_l_and_converts_back() {
    for range in [LRange::DEFAULT].iter().chain(&RANGES) {
        check_gpu_combine(REPLACE_L_FILE, *range, replace_l_expected);
    }
}

negative_control!(
    combiner_replace_l_wgsl_converts_sets_l_and_converts_back,
    "Replace-L that keeps the colour's own L must miss the composition at L = b",
    expected = "GPU combine",
    check_gpu_combine(
        File {
            text: &REPLACE_L.replace(
                "fn combine(rgb: vec3<f32>, b: f32) -> vec3<f32> { return oklab_to_linear(replace_l_lab(linear_to_oklab(rgb), b)); }",
                "fn combine(rgb: vec3<f32>, b: f32) -> vec3<f32> { return rgb; }",
            ),
            step: "replace_l_lab(x, b)",
        },
        LRange::DEFAULT,
        replace_l_expected
    )
);

#[test]
fn combiner_replace_l_wgsl_multiply_is_rgb_times_b() {
    check_gpu_combine(MULTIPLY_FILE, LRange::DEFAULT, multiply_expected);
}

negative_control!(
    combiner_replace_l_wgsl_multiply_is_rgb_times_b,
    "Multiply in encoded sRGB must miss rgb·b",
    expected = "GPU combine",
    check_gpu_combine(
        File {
            text: "fn combine(rgb: vec3<f32>, b: f32) -> vec3<f32> { return srgb_to_linear(linear_to_srgb(rgb) * b); }",
            step: "x",
        },
        LRange::DEFAULT,
        multiply_expected
    )
);

// ── REQ-COL-009: L-ownership ─────────────────────────────────────────────────────────────────────────────────────

/// The bound on OKLab recovered from a combiner's linear output through the forward map. Ottosson's published
/// matrices (dd_colouring §3.1) are inverses of each other only to their ten decimals, so the f64 round trip is off
/// by their residue, not by f64 rounding: white maps to `b_ab = 3.73·10⁻⁸`. Across an in-gamut output the residue
/// stays below `10⁻⁷`, five orders below an 8-bit display step (`1/255`).
const TOL_LAB: f64 = 1e-7;

/// A Replace-L, as a subject: `(range, colour, b)` to the OKLab colour it gives and that colour as linear sRGB.
type ReplaceL = fn(LRange, [f64; 3], f64) -> ([f64; 3], [f64; 3]);

/// Under `replace_l`, for every colour, brightness and range: the OKLab colour it gives has L = the brightness's
/// lightness and the colour's `(a, b_ab)`, bit for bit, and the output is that OKLab colour as linear sRGB, bit for
/// bit, so its L is the brightness's whatever the colour. Where the output is in gamut, what the display shows
/// unclamped, its OKLab recovered through the forward map is within [`TOL_LAB`] of it. (Out of gamut the display
/// stage's clamp replaces it, and near L = 0 the cube root magnifies the matrices' residue.)
fn check_l_ownership(replace_l: ReplaceL) {
    for range in [LRange::DEFAULT].iter().chain(&RANGES) {
        for b in BRIGHTNESSES {
            let l = range.lightness(b);
            for rgb in colours() {
                let c = space::linear_to_oklab(rgb);
                let (lab, out) = replace_l(*range, rgb, b);
                assert!(
                    lab[0].to_bits() == l.to_bits(),
                    "L = {} for colour {rgb:?} at b = {b}, {range:?}; brightness owns L = {l}",
                    lab[0]
                );
                assert!(
                    lab[1].to_bits() == c[1].to_bits() && lab[2].to_bits() == c[2].to_bits(),
                    "(a, b_ab) = {:?} for colour {rgb:?}, whose own are {:?}",
                    &lab[1..],
                    &c[1..]
                );
                let want = space::oklab_to_linear(lab);
                assert!(
                    out.map(f64::to_bits) == want.map(f64::to_bits),
                    "the output {out:?} is not its OKLab {lab:?} as linear sRGB, {want:?}"
                );
                if out.iter().all(|x| (0.0..=1.0).contains(x)) {
                    let back = space::linear_to_oklab(out);
                    assert!(
                        (back[0] - l).abs() <= TOL_LAB,
                        "the output's L is {} for colour {rgb:?} at b = {b}, {range:?}; brightness owns L = {l}",
                        back[0]
                    );
                }
            }
        }
    }
}

fn mirror_replace_l(range: LRange, rgb: [f64; 3], b: f64) -> ([f64; 3], [f64; 3]) {
    (
        combine::replace_l_oklab(range, rgb, b),
        combine::replace_l(range, rgb, b),
    )
}

#[test]
fn combiner_l_ownership_brightness_owns_l() {
    check_l_ownership(mirror_replace_l);
}

negative_control!(
    combiner_l_ownership_brightness_owns_l,
    "a lightness blended half from the colour's own L must not be owned by the brightness",
    expected = "brightness owns L",
    check_l_ownership(|range, rgb, b| {
        let own = space::linear_to_oklab(rgb)[0];
        let mut lab = combine::replace_l_oklab(range, rgb, b);
        lab[0] = 0.5 * (lab[0] + own);
        (lab, space::oklab_to_linear(lab))
    })
);

/// The same through `combine` with both slots bound: whatever the colour occupant gives, L is the brightness's.
#[test]
fn combiner_l_ownership_through_combine() {
    check_l_ownership(|range, rgb, b| {
        (
            combine::replace_l_oklab(range, rgb, b),
            combine::combine(Combiner::ReplaceL(range), Some(rgb), Some(b)),
        )
    });
}

negative_control!(
    combiner_l_ownership_through_combine,
    "Multiply through combine keeps the colour's own L structure and scales its chroma, so the brightness does not \
     own L nor the colour the hue and chroma",
    expected = "for colour",
    check_l_ownership(|_, rgb, b| {
        let out = combine::combine(Combiner::Multiply, Some(rgb), Some(b));
        (space::linear_to_oklab(out), out)
    })
);

// ── REQ-COL-023: None is the identity of combine ─────────────────────────────────────────────────────────────────

/// One row of the table under `combiner`: the colour and brightness, each present or None, and the result `combine`
/// gives as linear sRGB.
type Combine = fn(Combiner, Option<[f64; 3]>, Option<f64>) -> [f64; 3];

/// Each combination under Replace-L (default range and another) and Multiply, against colour_composition §4.1, bit
/// for bit: C + B → `OKLab(L = B, Cₐ, C_b)` as linear sRGB / `C · B`; C + None → C; None + B → white's OKLab with
/// L = B / `white · B`; None + None → `OKLab(0.6, 0, 0)` under both. White's OKLab is the table's `(·, 0, 0)` within
/// [`TOL_LAB`], the published matrices' residue.
fn check_truth_table(combine: Combine) {
    let mid_grey = space::oklab_to_linear([MID_GREY_L, 0.0, 0.0]);
    let white = space::linear_to_oklab(WHITE);
    assert!(
        white[1].abs() <= TOL_LAB && white[2].abs() <= TOL_LAB,
        "white's OKLab is {white:?}, not (1, 0, 0)"
    );
    let combiners = [
        Combiner::ReplaceL(LRange::DEFAULT),
        Combiner::ReplaceL(RANGES[0]),
        Combiner::Multiply,
    ];
    let same = |got: [f64; 3], want: [f64; 3]| got.map(f64::to_bits) == want.map(f64::to_bits);
    for combiner in combiners {
        let none = combine(combiner, None, None);
        assert!(
            same(none, mid_grey),
            "{combiner:?}: None + None = {none:?}; the table gives the mid-grey {mid_grey:?}"
        );
        for rgb in colours() {
            let got = combine(combiner, Some(rgb), None);
            assert!(
                same(got, rgb),
                "{combiner:?}: C + None = {got:?}; the table gives C = {rgb:?}"
            );
        }
        for b in BRIGHTNESSES {
            let grey = combine(combiner, None, Some(b));
            let want = match combiner {
                Combiner::ReplaceL(range) => {
                    space::oklab_to_linear([range.lightness(b), white[1], white[2]])
                }
                Combiner::Multiply => WHITE.map(|w| w * b),
            };
            assert!(
                same(grey, want),
                "{combiner:?}: None + {b} = {grey:?}; the table gives the greyscale {want:?}"
            );
            for rgb in colours() {
                let got = combine(combiner, Some(rgb), Some(b));
                let want = match combiner {
                    Combiner::ReplaceL(range) => {
                        let c = space::linear_to_oklab(rgb);
                        space::oklab_to_linear([range.lightness(b), c[1], c[2]])
                    }
                    Combiner::Multiply => rgb.map(|c| c * b),
                };
                assert!(
                    same(got, want),
                    "{combiner:?}: {rgb:?} + {b} = {got:?}; the table gives {want:?}"
                );
            }
        }
    }
}

#[test]
fn combiner_none_identity_truth_table() {
    check_truth_table(combine::combine);
}

negative_control!(
    combiner_none_identity_truth_table,
    "a combine that gives black for colour None and brightness None must miss the mid-grey",
    expected = "the table gives the mid-grey",
    check_truth_table(|k, c, b| match (c, b) {
        (None, None) => [0.0; 3],
        _ => combine::combine(k, c, b),
    })
);

/// `statement` for each combination: the combiner on both; the colour as it is; the combiner on white; the mid-grey.
fn check_statements(statement: fn(&str, bool, bool) -> String) {
    let want = [
        (true, true, "var out = n3_combine(rgb, b);"),
        (true, false, "var out = rgb;"),
        (false, true, "var out = n3_combine(vec3<f32>(1.0), b);"),
        (false, false, "var out = ramp_grey(0.6);"),
    ];
    for (colour, brightness, w) in want {
        let got = statement("n3_combine", colour, brightness);
        assert!(
            got.starts_with(w),
            "colour {colour}, brightness {brightness}: `{got}`, want `{w}`"
        );
    }
}

#[test]
fn combiner_none_identity_shade_statement() {
    check_statements(codegen::combine::statement);
}

negative_control!(
    combiner_none_identity_shade_statement,
    "a statement that calls the combiner with a None brightness as 0 must miss `rgb`",
    expected = "want `var out = rgb;`",
    check_statements(|c, colour, brightness| match (colour, brightness) {
        (true, false) => format!("var out = {c}(rgb, 0.0);"),
        _ => codegen::combine::statement(c, colour, brightness),
    })
);

// ── REQ-COL-023 on the GPU: the assembled stain ──────────────────────────────────────────────────────────────────

/// A source of nothing: the colour and brightness below read their uniforms, and a wired source makes them live.
const SOURCE: &str = "fn source(ctx: Ctx) -> Field { return Field(0.0, 0.0, 0.0, 0.0); }";

/// A colour of its uniforms.
const COLOUR: &str =
    "// @uniform c0: f32 = 0.0\n// @uniform c1: f32 = 0.0\n// @uniform c2: f32 = 0.0\n\
fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(uniforms.c0, uniforms.c1, uniforms.c2); }";

/// A brightness of its uniform.
const BRIGHTNESS: &str =
    "// @uniform v: f32 = 0.0\nfn brightness(ctx: Ctx) -> f32 { return uniforms.v; }";

/// The entry appended to the assembled stain: pixel 0 is `shade()`'s colour; pixel 1 is the table's result for the
/// same colour and brightness, from the storage words (row, C, B, L), written here through the shared library: row 0
/// C + B under Replace-L, linear → OKLab, L ← L, → linear; row 1 C + B under Multiply, C · B; row 2 C + None, C; row
/// 3 None + B under Replace-L, the same on white; row 4 None + B under Multiply, white · B. Row 5, None + None, is
/// checked in OKLab instead ([`MID_GREY_ROW`]).
const TABLE_ENTRY: &str = r"
@group(2) @binding(0) var<storage, read> table: array<u32>;

@fragment
fn t_table(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<u32> {
    var ctx: Ctx;
    ctx.frag_xy = pos.xy;
    var rgb = shade(ctx);
    if u32(pos.x) == 1u {
        let c = bitcast<vec3<f32>>(vec3<u32>(table[1], table[2], table[3]));
        let b = bitcast<f32>(table[4]);
        let l = bitcast<f32>(table[5]);
        let white = vec3<f32>(1.0);
        switch table[0] {
            case 0u: { rgb = oklab_to_linear(vec3<f32>(l, linear_to_oklab(c).yz)); }
            case 1u: { rgb = c * b; }
            case 2u: { rgb = c; }
            case 3u: { rgb = oklab_to_linear(vec3<f32>(l, linear_to_oklab(white).yz)); }
            case 4u: { rgb = white * b; }
            default: {}
        }
    }
    return vec4<u32>(bitcast<vec3<u32>>(rgb), 0u);
}
";

/// The stain source → colour, brightness → `combiner` → OUT, the colour and brightness each live or None.
fn table_stain(combiner: &str, colour: bool, brightness: bool) -> Stain {
    let custom = |live: bool, text: &str| {
        if live {
            Occupant::Custom(text.to_owned())
        } else {
            Occupant::None
        }
    };
    Stain::new(vec![
        Node {
            kind: Kind::Source,
            occupant: Occupant::Custom(SOURCE.into()),
            inputs: vec![],
        },
        Node {
            kind: Kind::Colour,
            occupant: custom(colour, COLOUR),
            inputs: vec![Some(0)],
        },
        Node {
            kind: Kind::Brightness,
            occupant: custom(brightness, BRIGHTNESS),
            inputs: vec![Some(0)],
        },
        Node {
            kind: Kind::Combiner,
            occupant: Occupant::BuiltIn(combiner.into()),
            inputs: vec![Some(1), Some(2)],
        },
        Node {
            kind: Kind::Out,
            occupant: Occupant::None,
            inputs: vec![Some(3)],
        },
    ])
    .unwrap_or_else(|e| panic!("{e}"))
}

/// The table's row None + None, the flat mid-grey. Its `ramp_grey(0.6)` is a constant expression, which the shader
/// compiler may evaluate in another order than a reference written here (the two differed by an ulp on Metal), so the
/// row is checked as OKLab: the GPU's f32 colour, through the f64 forward map, within [`TOL_F32`] of `(0.6, 0, 0)`.
const MID_GREY_ROW: u32 = 5;

/// The bound on an f32 GPU colour's OKLab against the table's: a few f32 ε (`1.19·10⁻⁷`) of the GPU's evaluation
/// (`tests/oklab_transcription.rs` bounds it per map) plus the matrices' residue within [`TOL_LAB`].
const TOL_F32: f64 = 1e-6;

/// Whether two f32 GPU colours, through the f64 forward map, are within [`TOL_F32`] of each other in OKLab: two
/// evaluations of one expression, which the shader compiler may order differently.
fn near_in_oklab(got: [f32; 3], want: [f32; 3]) -> bool {
    let (g, w) = (
        space::linear_to_oklab(wide(got)),
        space::linear_to_oklab(wide(want)),
    );
    (0..3).all(|k| (g[k] - w[k]).abs() <= TOL_F32)
}

/// The table's row for a combination under `combiner`.
fn row(combiner: Combiner, colour: bool, brightness: bool) -> u32 {
    match (combiner, colour, brightness) {
        (Combiner::ReplaceL(_), true, true) => 0,
        (Combiner::Multiply, true, true) => 1,
        (_, true, false) => 2,
        (Combiner::ReplaceL(_), false, true) => 3,
        (Combiner::Multiply, false, true) => 4,
        (_, false, false) => 5,
    }
}

/// Each combination under Replace-L at its default range and under Multiply, drawn: the assembled `shade()`
/// (`combiner_id` the built-in it names) gives the table's result, for a spread of colours and brightnesses: bit for
/// bit where the result is exact arithmetic (`C · B`, `C`, `white · B`), and where it goes through OKLab within
/// [`TOL_F32`] of the reference in OKLab ([`near_in_oklab`]). At the default range L is `b` exactly on the GPU (`combiner_replace_l_wgsl_step`), so the table's L
/// is the storage's `b`.
fn check_gpu_table(combiner_id: fn(Combiner) -> &'static str) {
    let gpu = GpuHarness::new().expect("a GPU device");
    let palette: Vec<[f32; 3]> = colours().into_iter().step_by(23).map(f32s).collect();
    for combiner in [Combiner::ReplaceL(LRange::DEFAULT), Combiner::Multiply] {
        for (colour, brightness) in [(true, true), (true, false), (false, true), (false, false)] {
            let stain = table_stain(combiner_id(combiner), colour, brightness);
            let fragment = assemble::assemble(&stain, Tier::FULL).unwrap_or_else(|e| panic!("{e}"));
            let source = fragment.source.clone() + TABLE_ENTRY;
            let mut group0 = vec![BindingKind::Uniform];
            group0.extend(fragment.uniforms.iter().map(|_| BindingKind::Uniform));
            let kernel = gpu
                .fragment(
                    &source,
                    "t_table",
                    &[&group0, &[], &[BindingKind::Storage]],
                    2,
                    1,
                )
                .unwrap_or_else(|e| panic!("{e}"));
            for &c in &palette {
                for b in BRIGHTNESSES.map(|b| b as f32) {
                    let range = match combiner {
                        Combiner::ReplaceL(r) => r,
                        Combiner::Multiply => LRange::DEFAULT,
                    };
                    // Each block by its first uniform's name: the canonical form renumbers the nodes.
                    let blocks: Vec<[u32; 4]> = fragment
                        .uniforms
                        .iter()
                        .map(|u| match u.uniforms[0].name.as_str() {
                            "c0" => [c[0].to_bits(), c[1].to_bits(), c[2].to_bits(), 0],
                            "v" => [b.to_bits(), 0, 0, 0],
                            "l_min" => [
                                (range.l_min as f32).to_bits(),
                                (range.l_max as f32).to_bits(),
                                0,
                                0,
                            ],
                            other => panic!("no block has a first uniform `{other}`"),
                        })
                        .collect();
                    let prelude_words = prelude::uniform_words(0);
                    let mut g0: Vec<&[u32]> = vec![&prelude_words];
                    g0.extend(blocks.iter().map(|w| &w[..]));
                    let words = [
                        row(combiner, colour, brightness),
                        c[0].to_bits(),
                        c[1].to_bits(),
                        c[2].to_bits(),
                        b.to_bits(),
                        range.lightness(b).to_bits(),
                    ];
                    let px = kernel
                        .draw(&[&g0, &[], &[&words]])
                        .unwrap_or_else(|e| panic!("{e}"));
                    let (got, want) = (&px[0][..3], &px[1][..3]);
                    let shown = wide([got[0], got[1], got[2]].map(f32::from_bits));
                    if row(combiner, colour, brightness) == MID_GREY_ROW {
                        let lab = space::linear_to_oklab(shown);
                        assert!(
                            (lab[0] - MID_GREY_L).abs() <= TOL_F32
                                && lab[1].abs() <= TOL_F32
                                && lab[2].abs() <= TOL_F32,
                            "{combiner:?}, colour None, brightness None: shade() is OKLab {lab:?}; the table gives \
                             (0.6, 0, 0)"
                        );
                        continue;
                    }
                    let exact = matches!(row(combiner, colour, brightness), 1 | 2 | 4);
                    let to_f32 = |w: &[u32]| [w[0], w[1], w[2]].map(f32::from_bits);
                    assert!(
                        if exact {
                            got == want
                        } else {
                            near_in_oklab(to_f32(got), to_f32(want))
                        },
                        "{combiner:?}, colour {}, brightness {}: shade() = {:?}; the table gives {:?}",
                        if colour { format!("{c:?}") } else { "None".into() },
                        if brightness { format!("{b}") } else { "None".into() },
                        wide([got[0], got[1], got[2]].map(f32::from_bits)),
                        wide([want[0], want[1], want[2]].map(f32::from_bits)),
                    );
                }
            }
        }
    }
}

#[test]
fn combiner_none_identity_on_the_gpu() {
    check_gpu_table(Combiner::id);
}

negative_control!(
    combiner_none_identity_on_the_gpu,
    "the pass-through combiner in place of Replace-L and Multiply must miss the table",
    expected = "the table gives",
    check_gpu_table(|_| "pass_through")
);
