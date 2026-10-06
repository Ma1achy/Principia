//! qa's tests for TASK-M7-04 (REQ-COL-038, REQ-COL-009, REQ-COL-023; R-77), written from the requirements and their
//! sources, dd_colouring §3.5 ("Combiners"), colour_composition §4.1's truth table, render_gui_spec §13
//! ("Occupant identity (None)") and R-77, not from the implementation.
//!
//! The colours the combiners give are read back to OKLab through a reference written here from Ottosson's listing
//! (`linear_srgb_to_oklab`, `oklab_to_linear_srgb`, the same revision qa transcribed for TASK-M7-02:
//! `bottosson/bottosson.github.io`, `posts/oklab/index.html`, commit `7561fbab5c8b982020ed212aebb0b8620c44b228`), so a
//! wrong conversion in the crate cannot hide behind itself.
//!
//! Tolerances: where the requirement says "exactly" or "bit-stable" (Replace-L's L = b at the defaults; (a, b_ab)
//! untouched) and where the result is one IEEE operation or none (Multiply's `rgb·b`; `C + None → C`), the check is
//! bit for bit. Where a colour goes linear sRGB → OKLab → linear sRGB, it is compared in linear sRGB (physical units,
//! inverse-encode contract Part 4: never read back to OKLab, ill-conditioned near L = 0) with the reference's
//! `oklab_to_linear_srgb` of the tabled OKLab, and the bound is
//! REQ-COL-049's round-trip tolerance as `fixtures/gates/oklab-roundtrip/gate.json` gives it (provisional until the
//! M7 gate, R-71, R-182): this file reads it, it does not choose it.
//!
//! Every check takes its subject as an argument, and its negative control runs it on a wrong one (R-176).

use std::path::Path;

use ledger::gen::prelude;
use render::assemble::{self, Kind, Node, Occupant, Stain, Tier};
use render::colour::combine::{self, Combiner, LRange};
use render::colour::space;
use validation::gpu::{BindingKind, GpuHarness};
use validation::negative_control;

// ── The reference OKLab, Ottosson's listing ────────────────────────────────────────────────────────────────────

type Mat3 = [[f64; 3]; 3];

const O_M1: Mat3 = [
    [0.4122214708, 0.5363325363, 0.0514459929],
    [0.2119034982, 0.6806995451, 0.1073969566],
    [0.0883024619, 0.2817188376, 0.6299787005],
];
const O_M2: Mat3 = [
    [0.2104542553, 0.7936177850, -0.0040720468],
    [1.9779984951, -2.4285922050, 0.4505937099],
    [0.0259040371, 0.7827717662, -0.8086757660],
];
const O_M2_INV: Mat3 = [
    [1.0, 0.3963377774, 0.2158037573],
    [1.0, -0.1055613458, -0.0638541728],
    [1.0, -0.0894841775, -1.2914855480],
];
const O_M1_INV: Mat3 = [
    [4.0767416621, -3.3077115913, 0.2309699292],
    [-1.2684380046, 2.6097574011, -0.3413193965],
    [-0.0041960863, -0.7034186147, 1.7076147010],
];

fn mul(m: &Mat3, v: [f64; 3]) -> [f64; 3] {
    m.map(|r| r[0] * v[0] + r[1] * v[1] + r[2] * v[2])
}

/// Ottosson's `linear_srgb_to_oklab`, f64.
fn ref_lab(rgb: [f64; 3]) -> [f64; 3] {
    mul(&O_M2, mul(&O_M1, rgb).map(f64::cbrt))
}

/// Ottosson's `oklab_to_linear_srgb`, f64.
fn ref_rgb(lab: [f64; 3]) -> [f64; 3] {
    mul(&O_M1_INV, mul(&O_M2_INV, lab).map(|x| x * x * x))
}

fn wide(x: [f32; 3]) -> [f64; 3] {
    x.map(f64::from)
}

fn narrow(x: [f64; 3]) -> [f32; 3] {
    x.map(|c| c as f32)
}

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

/// REQ-COL-049's round-trip tolerance, read from the gate's configuration, with its requirement checked.
fn tolerance() -> f64 {
    let config = validation::gate::config(root(), "oklab-roundtrip")
        .expect("the oklab-roundtrip gate config");
    assert_eq!(config.threshold.requirement, "REQ-COL-049");
    config.threshold.value
}

fn near(got: [f64; 3], want: [f64; 3], tol: f64) -> bool {
    (0..3).all(|k| (got[k] - want[k]).abs() <= tol)
}

// ── Inputs ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// Linear-sRGB colours across the gamut: the primaries and secondaries, greys from black to white, dark and light
/// tints, and a 5³ lattice of the cube's interior.
fn colours() -> Vec<[f64; 3]> {
    let mut out = vec![
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 1.0],
        [1.0, 0.0, 1.0],
        [0.0, 0.0, 0.0],
        [0.05, 0.05, 0.05],
        [0.5, 0.5, 0.5],
        [1.0, 1.0, 1.0],
        [0.02, 0.0, 0.04],
        [0.9, 0.95, 0.8],
    ];
    let steps = [0.1, 0.3, 0.5, 0.7, 0.9];
    for r in steps {
        for g in steps {
            for b in steps {
                out.push([r, g, b]);
            }
        }
    }
    out
}

/// Brightnesses: both ends, values exact in binary and values that are not.
const BS: [f64; 11] = [
    0.0,
    0.1,
    0.2,
    0.25,
    1.0 / 3.0,
    0.5,
    0.6,
    0.7,
    0.875,
    0.99,
    1.0,
];

/// Non-default Replace-L ranges (the range form is the general case, R-77).
const RANGES: [LRange; 3] = [
    LRange {
        l_min: 0.2,
        l_max: 0.9,
    },
    LRange {
        l_min: 0.35,
        l_max: 0.65,
    },
    LRange {
        l_min: 0.1,
        l_max: 0.4,
    },
];

// ── REQ-COL-038: Replace-L and Multiply (dd_colouring §3.5, unit test 6) ───────────────────────────────────────

/// R-77: the defaults are L_min = 0 and L_max = 1, for the range `subject` gives as the default.
fn check_defaults(subject: LRange) {
    assert!(
        subject
            == LRange {
                l_min: 0.0,
                l_max: 1.0
            },
        "Replace-L's default range is {subject:?}; R-77 gives L_min = 0, L_max = 1"
    );
}

#[test]
fn combiner_replace_l_qa_defaults_are_zero_and_one() {
    check_defaults(LRange::default());
    check_defaults(LRange::DEFAULT);
}

negative_control!(
    combiner_replace_l_qa_defaults_are_zero_and_one,
    "a non-default range is not R-77's default",
    expected = "R-77 gives",
    check_defaults(RANGES[0])
);

/// Replace-L's OKLab output: `(colour, b) → OKLab` in f64 and in f32.
type Lab64 = fn(LRange, [f64; 3], f64) -> [f64; 3];
type Lab32 = fn(LRange, [f32; 3], f32) -> [f32; 3];

/// (a, b_ab) bit-stable: the OKLab Replace-L produces has the colour's own (a, b_ab), bit for bit, in f64 and f32,
/// at the default and at non-default ranges.
fn check_ab_bit_stable(f64_step: Lab64, f32_step: Lab32) {
    for range in std::iter::once(LRange::DEFAULT).chain(RANGES) {
        for rgb in colours() {
            let lab = space::linear_to_oklab(rgb);
            let lab32 = space::linear_to_oklab(narrow(rgb));
            for b in BS {
                let out = f64_step(range, rgb, b);
                let out32 = f32_step(range, narrow(rgb), b as f32);
                assert!(
                    out[1].to_bits() == lab[1].to_bits() && out[2].to_bits() == lab[2].to_bits(),
                    "a/b not bit-stable (f64): {rgb:?} b={b} {range:?}: {out:?} vs {lab:?}"
                );
                assert!(
                    out32[1].to_bits() == lab32[1].to_bits()
                        && out32[2].to_bits() == lab32[2].to_bits(),
                    "a/b not bit-stable (f32): {rgb:?} b={b} {range:?}: {out32:?} vs {lab32:?}"
                );
            }
        }
    }
}

#[test]
fn combiner_replace_l_qa_ab_bit_stable() {
    check_ab_bit_stable(combine::replace_l_oklab, combine::replace_l_oklab);
}

negative_control!(
    combiner_replace_l_qa_ab_bit_stable,
    "OKLab read back from Replace-L's RGB output is not bit-stable in (a, b_ab)",
    expected = "not bit-stable",
    check_ab_bit_stable(
        |r, c, b| space::linear_to_oklab(combine::replace_l(r, c, b)),
        |r, c, b| space::linear_to_oklab(combine::replace_l(r, c, b)),
    )
);

/// At the default range the output L equals b exactly (f64 and f32), and Replace-L's linear-sRGB output, read back
/// through Ottosson's reference, has L = b and the colour's (a, b_ab), within REQ-COL-049's tolerance.
fn check_default_l_is_b(
    f64_step: Lab64,
    f32_step: Lab32,
    rgb_out: fn(LRange, [f64; 3], f64) -> [f64; 3],
) {
    let tol = tolerance();
    for rgb in colours() {
        let lab = ref_lab(rgb);
        for b in BS
            .into_iter()
            .chain((0..=64).map(|i| f64::from(i) / 64.0 + 1e-3 * f64::from(i % 7)))
        {
            let b = b.min(1.0);
            let l = f64_step(LRange::DEFAULT, rgb, b)[0];
            assert!(
                l.to_bits() == b.to_bits(),
                "L != b exactly (f64): {rgb:?} b={b}: L={l}"
            );
            let l32 = f32_step(LRange::DEFAULT, narrow(rgb), b as f32)[0];
            assert!(
                l32.to_bits() == (b as f32).to_bits(),
                "L != b exactly (f32): {rgb:?} b={b}: L={l32}"
            );
            let got = rgb_out(LRange::DEFAULT, rgb, b);
            let want = ref_rgb([b, lab[1], lab[2]]);
            assert!(
                near(got, want, tol),
                "Replace-L output {got:?} is not OKLab ({b}, {}, {}) = {want:?}",
                lab[1],
                lab[2]
            );
        }
    }
}

#[test]
fn combiner_replace_l_qa_default_l_is_b() {
    check_default_l_is_b(
        combine::replace_l_oklab,
        combine::replace_l_oklab,
        combine::replace_l,
    );
}

negative_control!(
    combiner_replace_l_qa_default_l_is_b,
    "L = b² in place of L = b breaks the default",
    expected = "L != b exactly",
    check_default_l_is_b(
        |r, c, b| combine::replace_l_oklab(r, c, b * b),
        combine::replace_l_oklab,
        combine::replace_l,
    )
);

negative_control!(
    combiner_replace_l_qa_default_l_is_b_rgb,
    "Multiply's output is not OKLab(b, Cₐ, C_b)",
    expected = "is not OKLab",
    check_default_l_is_b(
        combine::replace_l_oklab,
        combine::replace_l_oklab,
        |_, c, b| combine::multiply(c, b),
    )
);

/// A non-default range maps b = 0 to L_min and b = 1 to L_max, and b = ½ to their midpoint (the map is the affine
/// `L_min + (L_max − L_min)·b`); through the RGB output, read back by the reference, likewise.
fn check_range(step: Lab64, rgb_out: fn(LRange, [f64; 3], f64) -> [f64; 3]) {
    let tol = tolerance();
    for range in RANGES {
        for rgb in colours() {
            for (b, want) in [
                (0.0, range.l_min),
                (1.0, range.l_max),
                (0.5, 0.5 * (range.l_min + range.l_max)),
                (0.25, range.l_min + 0.25 * (range.l_max - range.l_min)),
            ] {
                let l = step(range, rgb, b)[0];
                assert!(
                    (l - want).abs() <= f64::EPSILON,
                    "range {range:?} maps b={b} to L={l}; want {want}"
                );
                let lab = ref_lab(rgb);
                let (got, want_rgb) = (rgb_out(range, rgb, b), ref_rgb([want, lab[1], lab[2]]));
                assert!(
                    near(got, want_rgb, tol),
                    "range {range:?} maps b={b} to {got:?}; want OKLab L={want}, {want_rgb:?}"
                );
            }
        }
    }
}

#[test]
fn combiner_replace_l_qa_range_maps_the_ends() {
    check_range(combine::replace_l_oklab, combine::replace_l);
}

negative_control!(
    combiner_replace_l_qa_range_maps_the_ends,
    "a combiner that ignores the range (always L = b) misses L_min/L_max",
    expected = "range",
    check_range(
        |_, c, b| combine::replace_l_oklab(LRange::DEFAULT, c, b),
        combine::replace_l,
    )
);

/// Multiply is `rgb·b` in linear space: each channel bit for bit the IEEE product, so the channel ratios hold, in f64
/// and f32.
fn check_multiply(f64_m: fn([f64; 3], f64) -> [f64; 3], f32_m: fn([f32; 3], f32) -> [f32; 3]) {
    for rgb in colours() {
        for b in BS {
            let out = f64_m(rgb, b);
            let out32 = f32_m(narrow(rgb), b as f32);
            for k in 0..3 {
                assert!(
                    out[k].to_bits() == (rgb[k] * b).to_bits(),
                    "Multiply is not rgb·b (f64): {rgb:?}·{b} = {out:?}"
                );
                assert!(
                    out32[k].to_bits() == (rgb[k] as f32 * b as f32).to_bits(),
                    "Multiply is not rgb·b (f32): {rgb:?}·{b} = {out32:?}"
                );
            }
            if b > 0.0 {
                for (i, j) in [(0, 1), (1, 2), (0, 2)] {
                    if rgb[j] > 0.0 {
                        let (r_in, r_out) = (rgb[i] / rgb[j], out[i] / out[j]);
                        assert!(
                            ((r_out - r_in) / r_in.max(f64::MIN_POSITIVE)).abs()
                                <= 4.0 * f64::EPSILON
                                || (r_out - r_in).abs() <= 4.0 * f64::EPSILON,
                            "Multiply is not rgb·b: channel ratio {i}/{j} {r_in} became {r_out}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn combiner_replace_l_qa_multiply_is_linear_rgb_times_b() {
    check_multiply(combine::multiply, combine::multiply);
}

negative_control!(
    combiner_replace_l_qa_multiply_is_linear_rgb_times_b,
    "Multiply in encoded sRGB rather than linear space",
    expected = "Multiply is not rgb·b",
    check_multiply(
        |c, b| c.map(|x| space::srgb_to_linear(space::linear_to_srgb(x) * b)),
        combine::multiply,
    )
);

// ── REQ-COL-009: a bound brightness owns L ─────────────────────────────────────────────────────────────────────

type Combine64 = fn(Combiner, Option<[f64; 3]>, Option<f64>) -> [f64; 3];

/// With colour and brightness bound under Replace-L, the output's L (read back by the reference) is the
/// brightness-derived L whatever the colour, and the colour gives only hue and chroma: two colours of the same (a,
/// b_ab) and different L give the same output.
fn check_l_ownership(f: Combine64) {
    let tol = tolerance();
    for range in std::iter::once(LRange::DEFAULT).chain(RANGES) {
        let c = Combiner::ReplaceL(range);
        for b in BS {
            let want_l = range.l_min + (range.l_max - range.l_min) * b;
            for rgb in colours() {
                let lab = ref_lab(rgb);
                let (got, want) = (f(c, Some(rgb), Some(b)), ref_rgb([want_l, lab[1], lab[2]]));
                assert!(
                    near(got, want, tol),
                    "brightness does not own L: colour {rgb:?}, b={b}, {range:?}: {got:?}, want OKLab L={want_l}, \
                     {want:?}"
                );
            }
            // Same hue and chroma, different lightness: the colour's own L must not reach the output.
            for (a, bb) in [(0.05, 0.02), (-0.04, 0.06), (0.0, -0.08), (0.02, 0.0)] {
                let dark = ref_rgb([0.45, a, bb]);
                let light = ref_rgb([0.8, a, bb]);
                let (od, ol) = (f(c, Some(dark), Some(b)), f(c, Some(light), Some(b)));
                assert!(
                    near(od, ol, tol),
                    "brightness does not own L: colours of L 0.45 and 0.8 with (a,b)=({a},{bb}) give {od:?} and {ol:?} \
                     at b={b}"
                );
            }
        }
    }
}

#[test]
fn combiner_l_ownership_qa_regardless_of_colour() {
    check_l_ownership(combine::combine);
}

negative_control!(
    combiner_l_ownership_qa_regardless_of_colour,
    "Multiply keeps the colour's own L, so the brightness does not own it",
    expected = "brightness does not own L",
    check_l_ownership(|_, c, b| combine::combine(Combiner::Multiply, c, b))
);

// ── REQ-COL-023: None is the identity of combine (colour_composition §4.1) ─────────────────────────────────────

/// Each of the four combinations under Replace-L (default range) and Multiply gives the tabled result:
/// C+B → OKLab(B, Cₐ, C_b) / C·B; C+None → C / C·1; None+B → OKLab(B, 0, 0) / white·B; None+None → OKLab(0.6, 0, 0)
/// under both.
fn check_truth_table(f: Combine64) {
    let tol = tolerance();
    let rl = Combiner::ReplaceL(LRange::DEFAULT);
    let mul = Combiner::Multiply;
    let grey = [0.6, 0.0, 0.0];
    for rgb in colours() {
        let lab = ref_lab(rgb);
        for b in BS {
            // C + B
            let got = f(rl, Some(rgb), Some(b));
            assert!(
                near(got, ref_rgb([b, lab[1], lab[2]]), tol),
                "table row C+B, Replace-L: {rgb:?}, {b}: {got:?}"
            );
            let got = f(mul, Some(rgb), Some(b));
            assert!(
                got == rgb.map(|x| x * b),
                "table row C+B, Multiply: {rgb:?}, {b}: {got:?}"
            );
            // None + B
            let got = f(rl, None, Some(b));
            assert!(
                near(got, ref_rgb([b, 0.0, 0.0]), tol),
                "table row None+B, Replace-L: {b}: {got:?}"
            );
            let got = f(mul, None, Some(b));
            assert!(got == [b, b, b], "table row None+B, Multiply: {b}: {got:?}");
        }
        // C + None
        for c in [rl, mul, Combiner::ReplaceL(RANGES[0])] {
            let got = f(c, Some(rgb), None);
            assert!(got == rgb, "table row C+None, {c:?}: {rgb:?} gave {got:?}");
        }
    }
    // None + None
    let (g_rl, g_mul) = (f(rl, None, None), f(mul, None, None));
    assert!(
        near(g_rl, ref_rgb(grey), tol) && near(g_mul, ref_rgb(grey), tol),
        "table row None+None: {g_rl:?} / {g_mul:?}, want OKLab (0.6, 0, 0) = {:?}",
        ref_rgb(grey)
    );
    assert!(
        g_rl == g_mul,
        "table row None+None: the mid-grey differs by combiner: {g_rl:?} / {g_mul:?}"
    );
}

#[test]
fn combiner_none_identity_qa_truth_table() {
    check_truth_table(combine::combine);
}

negative_control!(
    combiner_none_identity_qa_truth_table,
    "brightness None read as 0 (black) is not the identity",
    expected = "table row",
    check_truth_table(|c, rgb, b| combine::combine(c, rgb, Some(b.unwrap_or(0.0))))
);

negative_control!(
    combiner_none_identity_qa_truth_table_none_colour,
    "colour None read as black is not the identity",
    expected = "table row",
    check_truth_table(|c, rgb, b| combine::combine(c, Some(rgb.unwrap_or([0.0; 3])), b))
);

negative_control!(
    combiner_none_identity_qa_truth_table_mid_grey,
    "both None as OKLab(0.5,0,0) is not the tabled mid-grey",
    expected = "None+None",
    check_truth_table(|c, rgb, b| match (rgb, b) {
        (None, None) => ref_rgb([0.5, 0.0, 0.0]),
        _ => combine::combine(c, rgb, b),
    })
);

// ── On the GPU: the assembled stain with the built-in combiners ────────────────────────────────────────────────

const SOURCE: &str = "fn source(ctx: Ctx) -> Field { return Field(0.0, 0.0, 0.0, 0.0); }";
const COLOUR: &str =
    "// @uniform qr: f32 = 0.0\n// @uniform qg: f32 = 0.0\n// @uniform qb: f32 = 0.0\n\
fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(uniforms.qr, uniforms.qg, uniforms.qb); }";
const BRIGHTNESS: &str =
    "// @uniform qv: f32 = 0.0\nfn brightness(ctx: Ctx) -> f32 { return uniforms.qv; }";

const ENTRY: &str = r"
@fragment
fn qa_shade(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<u32> {
    var ctx: Ctx;
    ctx.frag_xy = pos.xy;
    return vec4<u32>(bitcast<vec3<u32>>(shade(ctx)), 0u);
}
";

fn stain(combiner: &str, colour: bool, brightness: bool) -> Stain {
    let live = |on: bool, text: &str| {
        if on {
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
            occupant: live(colour, COLOUR),
            inputs: vec![Some(0)],
        },
        Node {
            kind: Kind::Brightness,
            occupant: live(brightness, BRIGHTNESS),
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

/// The expected colour of one case: exact (bit for bit, f32), or an OKLab colour the output must match in linear sRGB
/// (physical units, never read back to OKLab near L = 0) within the tolerance.
enum Want {
    Exact([f32; 3]),
    Lab([f64; 3]),
}

/// Draws `shade()` of the stain `combiner` (a built-in id) with each slot live or None, with Replace-L's range
/// `range` written to the combiner's uniforms where it has them, and checks it against the table:
/// `table(combiner_is_replace_l, colour, b)`.
fn check_gpu(id_for: fn(bool) -> &'static str, range: LRange) {
    let tol = tolerance();
    let gpu = GpuHarness::new().expect("a GPU device");
    let palette: Vec<[f32; 3]> = colours().into_iter().step_by(11).map(narrow).collect();
    for replace_l in [true, false] {
        for (colour, brightness) in [(true, true), (true, false), (false, true), (false, false)] {
            let fragment =
                assemble::assemble(&stain(id_for(replace_l), colour, brightness), Tier::FULL)
                    .unwrap_or_else(|e| panic!("{e}"));
            let source = fragment.source.clone() + ENTRY;
            let mut group0 = vec![BindingKind::Uniform];
            for (i, u) in fragment.uniforms.iter().enumerate() {
                assert_eq!(
                    (u.group, u.binding as usize),
                    (0, i + 1),
                    "uniform block layout"
                );
                group0.push(BindingKind::Uniform);
            }
            let kernel = gpu
                .fragment(&source, "qa_shade", &[&group0], 1, 1)
                .unwrap_or_else(|e| panic!("{e}\n{source}"));
            for &c in &palette {
                for b in BS.map(|b| b as f32) {
                    let blocks: Vec<[u32; 4]> = fragment
                        .uniforms
                        .iter()
                        .map(|u| {
                            let mut w = [0u32; 4];
                            for (k, x) in u.uniforms.iter().enumerate() {
                                w[k] = match x.name.as_str() {
                                    "qr" => c[0].to_bits(),
                                    "qg" => c[1].to_bits(),
                                    "qb" => c[2].to_bits(),
                                    "qv" => b.to_bits(),
                                    "l_min" => (range.l_min as f32).to_bits(),
                                    "l_max" => (range.l_max as f32).to_bits(),
                                    other => panic!("unexpected uniform `{other}`"),
                                };
                            }
                            w
                        })
                        .collect();
                    let pw = prelude::uniform_words(0);
                    let mut g0: Vec<&[u32]> = vec![&pw];
                    g0.extend(blocks.iter().map(|w| &w[..]));
                    let px = kernel.draw(&[&g0]).unwrap_or_else(|e| panic!("{e}"));
                    let got = [px[0][0], px[0][1], px[0][2]].map(f32::from_bits);
                    let l = range.l_min + (range.l_max - range.l_min) * f64::from(b);
                    let lab = ref_lab(wide(c));
                    let want = match (replace_l, colour, brightness) {
                        (true, true, true) => Want::Lab([l, lab[1], lab[2]]),
                        (false, true, true) => Want::Exact(c.map(|x| x * b)),
                        (_, true, false) => Want::Exact(c),
                        (true, false, true) => Want::Lab([l, 0.0, 0.0]),
                        (false, false, true) => Want::Exact([b; 3]),
                        (_, false, false) => Want::Lab([0.6, 0.0, 0.0]),
                    };
                    let ok = match want {
                        Want::Exact(w) => got == w,
                        Want::Lab(w) => near(wide(got), ref_rgb(w), tol),
                    };
                    assert!(
                        ok,
                        "GPU table: {} colour {} brightness {}: shade() = {got:?} (OKLab {:?}), {range:?}",
                        if replace_l { "Replace-L" } else { "Multiply" },
                        if colour { format!("{c:?}") } else { "None".into() },
                        if brightness { format!("{b}") } else { "None".into() },
                        ref_lab(wide(got)),
                    );
                }
            }
        }
    }
}

fn builtin(replace_l: bool) -> &'static str {
    if replace_l {
        "replace_l"
    } else {
        "multiply"
    }
}

#[test]
fn combiner_none_identity_qa_gpu_table() {
    check_gpu(builtin, LRange::DEFAULT);
}

#[test]
fn combiner_replace_l_qa_gpu_range() {
    for range in RANGES {
        check_gpu(builtin, range);
    }
}

negative_control!(
    combiner_none_identity_qa_gpu_table,
    "the combiners swapped miss the table",
    expected = "GPU table",
    check_gpu(|r| builtin(!r), LRange::DEFAULT)
);

negative_control!(
    combiner_replace_l_qa_gpu_range,
    "pass-through in place of Replace-L misses the range",
    expected = "GPU table",
    check_gpu(|r| if r { "pass_through" } else { "multiply" }, RANGES[0])
);

/// The shipped Replace-L occupant's lightness uniforms default to R-77's L_min = 0, L_max = 1: the built-in `id`'s
/// assembled uniforms.
fn check_shipped_defaults(id: &str) {
    let fragment =
        assemble::assemble(&stain(id, true, true), Tier::FULL).unwrap_or_else(|e| panic!("{e}"));
    let find = |name: &str| {
        fragment
            .uniforms
            .iter()
            .flat_map(|u| u.uniforms.iter())
            .find(|u| u.name == name)
            .unwrap_or_else(|| {
                panic!("shipped defaults: no uniform `{name}` in the assembled `{id}`")
            })
            .default
            .clone()
    };
    assert_eq!(find("l_min"), vec![0.0], "shipped defaults: L_min");
    assert_eq!(find("l_max"), vec![1.0], "shipped defaults: L_max");
}

#[test]
fn combiner_replace_l_qa_shipped_defaults() {
    check_shipped_defaults("replace_l");
}

negative_control!(
    combiner_replace_l_qa_shipped_defaults,
    "Multiply has no lightness range",
    expected = "shipped defaults",
    check_shipped_defaults("multiply")
);
