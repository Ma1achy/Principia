//! qa's tests for TASK-M7-02 (REQ-COL-033, REQ-COL-049; R-51; PIT-3), written from the requirements and
//! dd_colouring §3.1 and §5 unit test 1, not from the implementation.
//!
//! **The reference** (R-184): Björn Ottosson, "A perceptual color space for image processing",
//! <https://bottosson.github.io/posts/oklab/>, source `bottosson/bottosson.github.io`, file `posts/oklab/index.html`,
//! commit `7561fbab5c8b982020ed212aebb0b8620c44b228`. qa transcribed, from that revision, the code listing
//! "Converting from linear sRGB to Oklab" (`linear_srgb_to_oklab`, `oklab_to_linear_srgb`), the XYZ → LMS matrix of
//! the section "Converting from XYZ to Oklab", and the "Table of example XYZ and Oklab pairs" ("computed by
//! transforming the XYZ coordinates to Oklab and rounding to three decimals"). Nothing is fetched at run time.
//!
//! What is checked:
//! - every §3.1 coefficient, in dd_colouring, in `render::colour::space::OKLAB` and in the WGSL (the prelude's
//!   `oklab_to_linear`, `colour_space.wgsl`'s `linear_to_oklab`), equals Ottosson's, sign and all ten decimals;
//! - the Rust transforms in f64 agree with an f64 reference written here from Ottosson's listing, closely enough that
//!   a one-unit slip in the tenth decimal of any of the 36 matrix entries is detected (PIT-3, shown for each);
//! - Ottosson's published example pairs (the primaries and the white of XYZ) through the Rust, in f64 and f32, and
//!   through the WGSL on the GPU, to the table's three decimals; white → (1, 0, 0), black → (0, 0, 0);
//! - the sRGB transfer at its two thresholds, which branch the threshold value itself takes, and its endpoints;
//! - OKLCH's `C = √(a² + b²)`, `h = atan2(b, a)` and its inverse;
//! - the f32 round trip on every 8-bit code of the sRGB cube's twelve edges and its grey diagonal, on the CPU and on
//!   the GPU, within REQ-COL-049's tolerance as `fixtures/gates/oklab-roundtrip/gate.json` gives it (provisional until
//!   the M7 gate, R-71, R-182): this test reads the value, it does not choose it.
//!
//! Every check takes its subject as an argument, and its negative control runs it on a wrong one (R-176).

use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};
use std::path::Path;

use ledger::gen::prelude;
use render::colour::space::{self, Coefficients, Mat3, OKLAB};
use validation::gpu::{BindingKind, GpuHarness};
use validation::negative_control;

// ── Ottosson's values, transcribed by qa from the pinned revision ───────────────────────────────────────────────

/// `linear_srgb_to_oklab`'s `l`, `m`, `s` rows: linear sRGB → LMS.
const O_M1: Mat3 = [
    [0.4122214708, 0.5363325363, 0.0514459929],
    [0.2119034982, 0.6806995451, 0.1073969566],
    [0.0883024619, 0.2817188376, 0.6299787005],
];

/// `linear_srgb_to_oklab`'s return rows: `lms^{1/3}` → OKLab.
const O_M2: Mat3 = [
    [0.2104542553, 0.7936177850, -0.0040720468],
    [1.9779984951, -2.4285922050, 0.4505937099],
    [0.0259040371, 0.7827717662, -0.8086757660],
];

/// `oklab_to_linear_srgb`'s `l_`, `m_`, `s_` rows: OKLab → `lms'`, `L`'s coefficient 1.
const O_M2_INV: Mat3 = [
    [1.0, 0.3963377774, 0.2158037573],
    [1.0, -0.1055613458, -0.0638541728],
    [1.0, -0.0894841775, -1.2914855480],
];

/// `oklab_to_linear_srgb`'s return rows: LMS → linear sRGB.
const O_M1_INV: Mat3 = [
    [4.0767416621, -3.3077115913, 0.2309699292],
    [-1.2684380046, 2.6097574011, -0.3413193965],
    [-0.0041960863, -0.7034186147, 1.7076147010],
];

/// The XYZ → LMS matrix of "Converting from XYZ to Oklab" (its `M₁`), used only to place the table's XYZ inputs in
/// linear sRGB.
const O_XYZ_TO_LMS: Mat3 = [
    [0.8189330101, 0.3618667424, -0.1288597137],
    [0.0329845436, 0.9293118715, 0.0361456387],
    [0.0482003018, 0.2643662691, 0.6338517070],
];

/// "Table of example XYZ and Oklab pairs": XYZ, then (L, a, b), rounded by its author to three decimals.
const O_TABLE: [([f64; 3], [f64; 3]); 4] = [
    ([0.950, 1.000, 1.089], [1.000, 0.000, 0.000]),
    ([1.000, 0.000, 0.000], [0.450, 1.236, -0.019]),
    ([0.000, 1.000, 0.000], [0.922, -0.671, 0.263]),
    ([0.000, 0.000, 1.000], [0.153, -1.415, -0.449]),
];

/// Half a unit of the table's third decimal: the most a value rounded to three decimals can differ from the exact one.
const TABLE_HALF_UNIT: f64 = 0.0005;

/// Ottosson's 33 written coefficients in the order his listing gives them: `M₁`, `M₂`, the six closed inverse
/// coefficients (not `L`'s 1), `M₁⁻¹`, each signed, ten decimals.
fn ottosson_sequence() -> Vec<String> {
    let mut out = Vec::new();
    let mut push = |m: &Mat3, skip_first_column: bool| {
        for row in m {
            for (j, x) in row.iter().enumerate() {
                if !(skip_first_column && j == 0) {
                    out.push(format!("{x:.10}"));
                }
            }
        }
    };
    push(&O_M1, false);
    push(&O_M2, false);
    push(&O_M2_INV, true);
    push(&O_M1_INV, false);
    out
}

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn read(rel: &str) -> String {
    let path = root().join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn mul(m: &Mat3, v: [f64; 3]) -> [f64; 3] {
    m.map(|r| r[0] * v[0] + r[1] * v[1] + r[2] * v[2])
}

fn max_abs_diff(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| (a[i] - b[i]).abs()).fold(0.0, f64::max)
}

/// REQ-COL-049's round-trip tolerance, read from the gate's configuration, with its requirement checked.
fn tolerance() -> f64 {
    let config = validation::gate::config(root(), "oklab-roundtrip")
        .expect("the oklab-roundtrip gate config");
    assert_eq!(
        config.threshold.requirement, "REQ-COL-049",
        "the round-trip tolerance must be REQ-COL-049's"
    );
    config.threshold.value
}

// ── 1. The coefficients, string for string (REQ-COL-033, R-51) ─────────────────────────────────────────────────

/// Every ten-decimal number in `text`, in order, signed: a `-` or `−` before it, spaces allowed between (a matrix
/// entry's sign, or the operator of a closed form `L − 0.1055613458a`).
fn ten_decimal_numbers(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let starts = chars[i].is_ascii_digit()
            && (i == 0 || !(chars[i - 1].is_ascii_digit() || chars[i - 1] == '.'));
        if !starts {
            i += 1;
            continue;
        }
        let mut j = i;
        while j < chars.len() && chars[j].is_ascii_digit() {
            j += 1;
        }
        let mut k = j;
        if k < chars.len() && chars[k] == '.' {
            k += 1;
            while k < chars.len() && chars[k].is_ascii_digit() {
                k += 1;
            }
        }
        if k - j == 11 {
            let mut b = i;
            while b > 0 && chars[b - 1] == ' ' {
                b -= 1;
            }
            let negative = b > 0 && (chars[b - 1] == '-' || chars[b - 1] == '−');
            let digits: String = chars[i..k].iter().collect();
            out.push(if negative {
                format!("-{digits}")
            } else {
                digits
            });
        }
        i = k.max(i + 1);
    }
    out
}

/// `text`'s ten-decimal numbers are Ottosson's 33, in his order, sign and digit.
fn check_sequence(what: &str, text: &str) {
    let got = ten_decimal_numbers(text);
    let want = ottosson_sequence();
    assert!(
        got == want,
        "{what}: its coefficients differ from Ottosson's\n got  {got:?}\n want {want:?}"
    );
}

/// The text of dd_colouring §3.1, up to §3.2.
fn section_3_1(doc: &str) -> String {
    let start = doc
        .find("### 3.1 Colour spaces")
        .expect("dd_colouring §3.1");
    let end = doc[start..].find("### 3.2").expect("dd_colouring §3.2") + start;
    doc[start..end].to_owned()
}

/// The body of the WGSL function `name`, from its `fn` line to the first line that is `}`.
fn wgsl_fn(source: &str, name: &str) -> String {
    let start = source
        .find(&format!("fn {name}("))
        .unwrap_or_else(|| panic!("no WGSL fn {name}"));
    let end = source[start..].find("\n}").expect("the fn's end") + start;
    source[start..end].to_owned()
}

/// The WGSL's coefficients: `linear_to_oklab`'s (M₁, M₂) then the prelude's `oklab_to_linear`'s (closed, M₁⁻¹).
fn wgsl_text(colour_space: &str, prelude: &str) -> String {
    format!(
        "{}\n{}",
        wgsl_fn(colour_space, "linear_to_oklab"),
        wgsl_fn(prelude, "oklab_to_linear")
    )
}

/// `k`'s four matrices are Ottosson's, entry for entry.
fn check_rust_coefficients(k: &Coefficients) {
    for (name, got, want) in [
        ("M₁", k.m1, O_M1),
        ("M₂", k.m2, O_M2),
        ("M₂⁻¹", k.m2_inv, O_M2_INV),
        ("M₁⁻¹", k.m1_inv, O_M1_INV),
    ] {
        assert!(
            got == want,
            "Rust {name} {got:?} is not Ottosson's {want:?}"
        );
    }
}

#[test]
fn qa_m7_02_rust_coefficients() {
    assert_eq!(
        ottosson_sequence().len(),
        33,
        "qa's transcription has 33 written coefficients"
    );
    check_rust_coefficients(&OKLAB);
}

negative_control!(
    qa_m7_02_rust_coefficients,
    "a one-digit slip in the Rust M₁ must fail",
    expected = "is not Ottosson's",
    check_rust_coefficients(&Coefficients {
        m1: [
            [0.4122214708, 0.5363325363, 0.0514459929],
            [0.2119034982, 0.6806995451, 0.1073969566],
            [0.0883024619, 0.2817188376, 0.6299787006],
        ],
        ..OKLAB
    })
);

fn doc_section() -> String {
    section_3_1(&read("docs/design/principia_dd_colouring.md"))
}

#[test]
fn qa_m7_02_doc_coefficients() {
    check_sequence("dd_colouring §3.1", &doc_section());
}

negative_control!(
    qa_m7_02_doc_coefficients,
    "dd_colouring with one sign of M₁⁻¹ dropped must fail",
    expected = "its coefficients differ from Ottosson's",
    check_sequence(
        "dd_colouring §3.1, a sign dropped",
        &doc_section().replacen("−0.7034186147", " 0.7034186147", 1)
    )
);

fn wgsl_sources(colour_space_from: &str, colour_space_to: &str) -> String {
    let colour_space = read("crates/render/shaders/wgsl/lib/colour_space.wgsl");
    if !colour_space_from.is_empty() {
        assert!(
            colour_space.contains(colour_space_from),
            "`{colour_space_from}` is not in colour_space.wgsl"
        );
    }
    wgsl_text(
        &colour_space.replace(colour_space_from, colour_space_to),
        &read(prelude::PATH),
    )
}

#[test]
fn qa_m7_02_wgsl_coefficients() {
    check_sequence("the WGSL", &wgsl_sources("", ""));
}

negative_control!(
    qa_m7_02_wgsl_coefficients,
    "a tenth-decimal slip in the WGSL's M₂ must fail the comparison",
    expected = "its coefficients differ from Ottosson's",
    check_sequence("the WGSL", &wgsl_sources("0.7827717662", "0.7827717663"))
);

// ── 2. The transforms against a reference written from Ottosson's listing (REQ-COL-033, PIT-3) ─────────────────

/// Ottosson's `linear_srgb_to_oklab`, in f64.
fn ref_forward(rgb: [f64; 3]) -> [f64; 3] {
    mul(&O_M2, mul(&O_M1, rgb).map(f64::cbrt))
}

/// Ottosson's `oklab_to_linear_srgb`, in f64.
fn ref_inverse(lab: [f64; 3]) -> [f64; 3] {
    mul(&O_M1_INV, mul(&O_M2_INV, lab).map(|x| x * x * x))
}

/// The linear-sRGB lattice `{0, 1/15, …, 1}³`.
fn rgb_lattice() -> Vec<[f64; 3]> {
    let v: Vec<f64> = (0..=15).map(|i| f64::from(i) / 15.0).collect();
    let mut out = Vec::new();
    for &r in &v {
        for &g in &v {
            for &b in &v {
                out.push([r, g, b]);
            }
        }
    }
    out
}

/// The f64 agreement bound with the reference: `4096·ε ≈ 9.1e-13`. Both sides run the same operations in f64 on
/// values below 8 in magnitude (M₁⁻¹'s largest absolute row sum is 7.6), so any reordering differs by a few ulps of 8,
/// ~1e-14; a one-unit slip in a coefficient's tenth decimal moves some output on the lattice by more than 2.6e-11
/// (shown for each of the 36 below), thirty times this bound.
const F64_BOUND: f64 = 4096.0 * f64::EPSILON;

/// The largest difference between `forward` and the reference over the lattice, and `inverse`'s over the lattice's
/// OKLab images.
fn deviation(
    forward: &dyn Fn([f64; 3]) -> [f64; 3],
    inverse: &dyn Fn([f64; 3]) -> [f64; 3],
) -> (f64, f64) {
    let mut f = 0.0f64;
    let mut i = 0.0f64;
    for rgb in rgb_lattice() {
        f = f.max(max_abs_diff(forward(rgb), ref_forward(rgb)));
        let lab = ref_forward(rgb);
        i = i.max(max_abs_diff(inverse(lab), ref_inverse(lab)));
    }
    (f, i)
}

fn check_against_reference(k: &Coefficients) {
    let (f, i) = deviation(&|x| space::linear_to_oklab_with(k, x), &|x| {
        space::oklab_to_linear_with(k, x)
    });
    assert!(
        f <= F64_BOUND && i <= F64_BOUND,
        "the f64 transforms differ from Ottosson's listing: forward by {f:e}, inverse by {i:e} (bound {F64_BOUND:e})"
    );
}

#[test]
fn qa_m7_02_f64_matches_ottossons_listing() {
    check_against_reference(&OKLAB);
    // The public maps are the same transforms through OKLAB.
    let (f, i) = deviation(
        &space::linear_to_oklab::<f64>,
        &space::oklab_to_linear::<f64>,
    );
    assert!(
        f <= F64_BOUND && i <= F64_BOUND,
        "linear_to_oklab / oklab_to_linear: {f:e}, {i:e}"
    );
    // PIT-3: the check can fire on a slip in the last written decimal of any of the 36 entries.
    for which in 0..4 {
        for r in 0..3 {
            for c in 0..3 {
                for by in [1e-10, -1e-10] {
                    let mut k = OKLAB;
                    let m = match which {
                        0 => &mut k.m1,
                        1 => &mut k.m2,
                        2 => &mut k.m2_inv,
                        _ => &mut k.m1_inv,
                    };
                    m[r][c] += by;
                    let (f, i) = deviation(&|x| space::linear_to_oklab_with(&k, x), &|x| {
                        space::oklab_to_linear_with(&k, x)
                    });
                    assert!(
                        f.max(i) > 10.0 * F64_BOUND,
                        "a slip of {by:e} in matrix {which} [{r}][{c}] moves the output by only {:e}",
                        f.max(i)
                    );
                }
            }
        }
    }
}

negative_control!(
    qa_m7_02_f64_matches_ottossons_listing,
    "a tenth-decimal slip in M₁⁻¹ must fail",
    expected = "differ from Ottosson's listing",
    check_against_reference(&Coefficients {
        m1_inv: [
            [4.0767416621, -3.3077115913, 0.2309699292],
            [-1.2684380046, 2.6097574011, -0.3413193965],
            [-0.0041960863, -0.7034186148, 1.7076147010],
        ],
        ..OKLAB
    })
);

// ── 3. Ottosson's published example pairs, white and black (dd_colouring §5 unit test 1) ───────────────────────

/// The table's XYZ in linear sRGB: `M₁⁻¹ · (XYZ → LMS)`, both Ottosson's.
fn table_rgb(xyz: [f64; 3]) -> [f64; 3] {
    mul(&O_M1_INV, mul(&O_XYZ_TO_LMS, xyz))
}

/// `to_lab` gives each of Ottosson's example pairs to the table's three decimals.
fn check_table(what: &str, to_lab: &dyn Fn([f64; 3]) -> [f64; 3]) {
    for (xyz, want) in O_TABLE {
        let got = to_lab(table_rgb(xyz));
        let d = max_abs_diff(got, want);
        assert!(
            d <= TABLE_HALF_UNIT,
            "{what}: XYZ {xyz:?} gives {got:?}, not Ottosson's {want:?} to three decimals (off by {d:e})"
        );
    }
}

/// White and black: sRGB white → (1, 0, 0) to the table's three decimals (its white, D65, is (1.000, 0.000, 0.000))
/// and back; black → (0, 0, 0) exactly, both ways.
fn check_white_black(
    what: &str,
    to_lab: &dyn Fn([f64; 3]) -> [f64; 3],
    to_rgb: &dyn Fn([f64; 3]) -> [f64; 3],
) {
    let w = to_lab([1.0; 3]);
    assert!(
        max_abs_diff(w, [1.0, 0.0, 0.0]) <= TABLE_HALF_UNIT,
        "{what}: white → {w:?}, not (1, 0, 0)"
    );
    let back = to_rgb([1.0, 0.0, 0.0]);
    assert!(
        max_abs_diff(back, [1.0; 3]) <= TABLE_HALF_UNIT,
        "{what}: (1, 0, 0) → {back:?}, not white"
    );
    assert!(
        to_lab([0.0; 3]) == [0.0; 3],
        "{what}: black → {:?}, not (0, 0, 0)",
        to_lab([0.0; 3])
    );
    assert!(
        to_rgb([0.0; 3]) == [0.0; 3],
        "{what}: (0, 0, 0) → {:?}, not black",
        to_rgb([0.0; 3])
    );
}

fn f32_map(f: fn([f32; 3]) -> [f32; 3]) -> impl Fn([f64; 3]) -> [f64; 3] {
    move |x| f(x.map(|v| v as f32)).map(f64::from)
}

#[test]
fn qa_m7_02_published_pairs_white_black() {
    check_table("f64", &space::linear_to_oklab::<f64>);
    check_table("f32", &f32_map(space::linear_to_oklab::<f32>));
    check_white_black(
        "f64",
        &space::linear_to_oklab::<f64>,
        &space::oklab_to_linear::<f64>,
    );
    check_white_black(
        "f32",
        &f32_map(space::linear_to_oklab::<f32>),
        &f32_map(space::oklab_to_linear::<f32>),
    );
    // Encoded white is linear white: the sRGB transfer's 1 is 1.
    let w = space::srgb_to_oklab([1.0f64; 3]);
    assert!(
        max_abs_diff(w, [1.0, 0.0, 0.0]) <= TABLE_HALF_UNIT,
        "encoded white → {w:?}"
    );
}

negative_control!(
    qa_m7_02_published_pairs_white_black,
    "a second-decimal slip in M₂'s a row must miss Ottosson's table",
    expected = "not Ottosson's",
    check_table("slipped", &|x| space::linear_to_oklab_with(
        &Coefficients {
            m2: [
                [0.2104542553, 0.7936177850, -0.0040720468],
                [1.9879984951, -2.4285922050, 0.4505937099],
                [0.0259040371, 0.7827717662, -0.8086757660],
            ],
            ..OKLAB
        },
        x
    ))
);

// ── 4. The sRGB transfer (dd_colouring §3.1) ───────────────────────────────────────────────────────────────────

/// `decode` is §3.1's: `c/12.92` at and below 0.04045, `((c+0.055)/1.055)^2.4` above; `encode` its inverse, `12.92c`
/// at and below 0.0031308. At each threshold the two branches differ (by 2.3e-9 and 2.9e-8), so which one the
/// threshold value takes is seen exactly.
fn check_transfer(decode: fn(f64) -> f64, encode: fn(f64) -> f64) {
    let lin = |c: f64| c / 12.92;
    let pow = |c: f64| ((c + 0.055) / 1.055).powf(2.4);
    let enc_lin = |c: f64| 12.92 * c;
    let enc_pow = |c: f64| 1.055 * c.powf(1.0 / 2.4) - 0.055;
    let close =
        |a: f64, b: f64| (a - b).abs() <= 8.0 * f64::EPSILON * b.abs().max(f64::MIN_POSITIVE);
    for c in [0.0, 0.01, 0.04045] {
        assert!(
            decode(c) == lin(c),
            "decode({c}) = {}, not c/12.92 = {}",
            decode(c),
            lin(c)
        );
    }
    for c in [0.04045 + 1e-12, 0.5, 0.9, 1.0] {
        assert!(
            close(decode(c), pow(c)),
            "decode({c}) = {}, not the power segment's {}",
            decode(c),
            pow(c)
        );
    }
    for c in [0.0, 0.001, 0.0031308] {
        assert!(
            encode(c) == enc_lin(c),
            "encode({c}) = {}, not 12.92c = {}",
            encode(c),
            enc_lin(c)
        );
    }
    for c in [0.0031308 + 1e-12, 0.2, 0.7, 1.0] {
        assert!(
            close(encode(c), enc_pow(c)),
            "encode({c}) = {}, not the power segment's {}",
            encode(c),
            enc_pow(c)
        );
    }
    assert!(
        close(decode(1.0), 1.0) && close(encode(1.0), 1.0),
        "the transfer does not fix 1"
    );
    for code in 0..=255u8 {
        let c = f64::from(code) / 255.0;
        let back = encode(decode(c));
        assert!(
            (back - c).abs() <= 64.0 * f64::EPSILON,
            "code {code}: the transfer's round trip gives {back}"
        );
    }
}

#[test]
fn qa_m7_02_srgb_transfer() {
    check_transfer(space::srgb_to_linear::<f64>, space::linear_to_srgb::<f64>);
    // f32: the threshold value itself is on the linear segment.
    assert!(
        space::srgb_to_linear(0.04045f32) == 0.04045f32 / 12.92f32,
        "f32 decode at 0.04045"
    );
    assert!(
        space::linear_to_srgb(0.0031308f32) == 12.92f32 * 0.0031308f32,
        "f32 encode at 0.0031308"
    );
}

negative_control!(
    qa_m7_02_srgb_transfer,
    "a decode whose threshold is strict must fail at 0.04045",
    expected = "not c/12.92",
    check_transfer(
        |c| if c < 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        },
        space::linear_to_srgb::<f64>
    )
);

// ── 5. OKLCH (dd_colouring §3.1: C = √(a²+b²), h = atan2(b, a)) ────────────────────────────────────────────────

fn check_oklch(to_lch: fn([f64; 3]) -> [f64; 3], to_lab: fn([f64; 3]) -> [f64; 3]) {
    let near = |a: f64, b: f64| (a - b).abs() <= 4.0 * f64::EPSILON * b.abs().max(1.0);
    for (lab, c, h) in [
        ([0.5, 3.0, 4.0], 5.0, 4f64.atan2(3.0)),
        ([0.7, -0.1, 0.0], 0.1, PI),
        ([0.7, 0.0, -0.2], 0.2, -FRAC_PI_2),
        ([0.3, 0.1, 0.1], 0.1 * 2f64.sqrt(), FRAC_PI_4),
        ([0.9, 0.0, 0.25], 0.25, FRAC_PI_2),
    ] {
        let got = to_lch(lab);
        assert!(
            got[0].to_bits() == lab[0].to_bits() && near(got[1], c) && near(got[2], h),
            "OKLCH of {lab:?} is {got:?}, not (L, {c}, {h})"
        );
        let back = to_lab([lab[0], c, h]);
        assert!(
            back[0].to_bits() == lab[0].to_bits()
                && (0..3).all(|i| (back[i] - lab[i]).abs() <= 4.0 * f64::EPSILON),
            "OKLab of (L, {c}, {h}) is {back:?}, not {lab:?}"
        );
    }
    for rgb in rgb_lattice() {
        let lab = ref_forward(rgb);
        let lch = to_lch(lab);
        assert!(
            (-PI..=PI).contains(&lch[2]),
            "h {} of {lab:?} is outside [−π, π]",
            lch[2]
        );
        let back = to_lab(lch);
        assert!(
            max_abs_diff(back, lab) <= 16.0 * f64::EPSILON,
            "OKLCH round trip of {lab:?} gives {back:?}"
        );
    }
}

#[test]
fn qa_m7_02_oklch() {
    check_oklch(space::oklab_to_oklch::<f64>, space::oklch_to_oklab::<f64>);
    let lch = space::oklab_to_oklch([0.5f32, 3.0, 4.0]);
    assert!(
        lch[1] == 5.0 && (lch[2] - 4f32.atan2(3.0)).abs() <= f32::EPSILON,
        "f32 OKLCH {lch:?}"
    );
}

negative_control!(
    qa_m7_02_oklch,
    "atan2 with its arguments swapped must fail",
    expected = "not (L,",
    check_oklch(
        |[l, a, b]| [l, (a * a + b * b).sqrt(), a.atan2(b)],
        space::oklch_to_oklab::<f64>
    )
);

// ── 6. The f32 round trip across the cube's edges and greys, within REQ-COL-049 (dd_colouring §5 test 1) ───────

/// Every 8-bit code on the sRGB cube's twelve edges, and its grey diagonal.
fn edge_codes() -> Vec<[u8; 3]> {
    let mut out = Vec::new();
    for t in 0..=255u8 {
        for a in [0u8, 255] {
            for b in [0u8, 255] {
                out.push([t, a, b]);
                out.push([a, t, b]);
                out.push([a, b, t]);
            }
        }
        out.push([t, t, t]);
    }
    out
}

/// The round trip `c → lin → OKLab → lin' → c'` in f32 through `k` on every edge code: the largest `|lin' − lin|` and
/// the largest `|c' − c|`.
fn round_trip_errors(k: &Coefficients) -> (f64, f64) {
    let mut el = 0.0f64;
    let mut es = 0.0f64;
    for code in edge_codes() {
        let c = code.map(|x| f32::from(x) / 255.0);
        let lin = c.map(space::srgb_to_linear::<f32>);
        let back = space::oklab_to_linear_with(k, space::linear_to_oklab_with(k, lin));
        let enc = back.map(space::linear_to_srgb::<f32>);
        for i in 0..3 {
            el = el.max((f64::from(back[i]) - f64::from(lin[i])).abs());
            es = es.max((f64::from(enc[i]) - f64::from(c[i])).abs());
        }
    }
    (el, es)
}

fn check_round_trip(k: &Coefficients, tol: f64) {
    let (el, es) = round_trip_errors(k);
    println!("CPU f32 round trip over the cube's edges and greys: linear {el:e}, encoded {es:e}, tolerance {tol:e}");
    assert!(
        el <= tol && es <= tol,
        "the f32 round trip exceeds REQ-COL-049's {tol:e}: linear {el:e}, encoded {es:e}"
    );
}

#[test]
fn qa_m7_02_round_trip_edges_cpu() {
    check_round_trip(&OKLAB, tolerance());
    // The public encoded chain is the same round trip.
    let tol = tolerance();
    for code in edge_codes() {
        let c = code.map(|x| f32::from(x) / 255.0);
        let back = space::oklab_to_srgb(space::srgb_to_oklab(c));
        let d = (0..3)
            .map(|i| (back[i] - c[i]).abs())
            .fold(0.0f32, f32::max);
        assert!(
            f64::from(d) <= tol,
            "srgb_to_oklab / oklab_to_srgb at {code:?}: {d:e}"
        );
    }
}

negative_control!(
    qa_m7_02_round_trip_edges_cpu,
    "M₁[0][0] wrong in its fourth decimal must break the round trip",
    expected = "exceeds REQ-COL-049",
    check_round_trip(
        &Coefficients {
            m1: [
                [0.4123214708, 0.5363325363, 0.0514459929],
                [0.2119034982, 0.6806995451, 0.1073969566],
                [0.0883024619, 0.2817188376, 0.6299787005],
            ],
            ..OKLAB
        },
        tolerance()
    )
);

// ── 7. The WGSL on the GPU: the published pairs, white, black and the round trip (REQ-COL-033) ──────────────────

const ENTRY: &str = r"
@group(2) @binding(0) var<storage, read> cases: array<vec4<u32>>;

const QA_WIDTH: u32 = 256u;

@fragment
fn qa_colour(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<u32> {
    let i = min(u32(pos.y) * QA_WIDTH + u32(pos.x), arrayLength(&cases) - 1u);
    let c = cases[i];
    let x = bitcast<vec3<f32>>(c.yzw);
    var r = vec3<f32>(0.0);
    switch c.x {
        case 1u: { r = linear_to_oklab(x); }
        case 2u: { r = oklab_to_linear(x); }
        case 3u: {
            let lin = srgb_to_linear(x);
            r = oklab_to_linear(linear_to_oklab(lin)) - lin;
        }
        case 4u: { r = oklab_to_srgb(srgb_to_oklab(x)) - x; }
        default: {}
    }
    return vec4<u32>(bitcast<vec3<u32>>(r), 0u);
}
";

const QA_WIDTH: usize = 256;

/// Each `(op, x)` through the checked-in prelude and `colour_space.wgsl`, with `(from, to)` replaced in the latter.
fn gpu(cases: &[(u32, [f32; 3])], replace: (&str, &str)) -> Vec<[f64; 3]> {
    let gpu = GpuHarness::new().expect("a GPU device");
    let colour_space = read("crates/render/shaders/wgsl/lib/colour_space.wgsl");
    let colour_space = if replace.0.is_empty() {
        colour_space
    } else {
        assert!(
            colour_space.contains(replace.0),
            "`{}` is not in colour_space.wgsl",
            replace.0
        );
        colour_space.replace(replace.0, replace.1)
    };
    let module = format!("{}\n{colour_space}\n{ENTRY}", read(prelude::PATH));
    let words: Vec<u32> = cases
        .iter()
        .flat_map(|(op, x)| [*op, x[0].to_bits(), x[1].to_bits(), x[2].to_bits()])
        .collect();
    let rows = cases.len().div_ceil(QA_WIDTH);
    let kernel = gpu
        .fragment(
            &module,
            "qa_colour",
            &[&[], &[], &[BindingKind::Storage]],
            QA_WIDTH as u32,
            rows as u32,
        )
        .unwrap_or_else(|e| panic!("{e}"));
    let pixels = kernel
        .draw(&[&[], &[], &[&words]])
        .unwrap_or_else(|e| panic!("{e}"));
    pixels[..cases.len()]
        .iter()
        .map(|p| [p[0], p[1], p[2]].map(|w| f64::from(f32::from_bits(w))))
        .collect()
}

fn check_gpu(replace: (&str, &str)) {
    let tol = tolerance();
    let mut cases: Vec<(u32, [f32; 3])> = O_TABLE
        .iter()
        .map(|(xyz, _)| (1, table_rgb(*xyz).map(|v| v as f32)))
        .collect();
    cases.extend([
        (1, [1.0; 3]),
        (2, [1.0, 0.0, 0.0]),
        (1, [0.0; 3]),
        (2, [0.0; 3]),
    ]);
    let fixed = cases.len();
    for code in edge_codes() {
        let c = code.map(|x| f32::from(x) / 255.0);
        cases.push((3, c));
        cases.push((4, c));
    }
    let out = gpu(&cases, replace);
    let (mut el, mut es) = (0.0f64, 0.0f64);
    for (j, r) in out[fixed..].iter().enumerate() {
        let e = r.iter().fold(0.0f64, |m, v| m.max(v.abs()));
        if j % 2 == 0 {
            el = el.max(e);
        } else {
            es = es.max(e);
        }
    }
    println!("GPU f32 round trip over the cube's edges and greys: linear {el:e}, encoded {es:e}, tolerance {tol:e}");
    assert!(
        el <= tol && es <= tol,
        "GPU: the round trip exceeds REQ-COL-049's {tol:e}: linear {el:e}, encoded {es:e}"
    );
    for (i, (_, want)) in O_TABLE.iter().enumerate() {
        let d = max_abs_diff(out[i], *want);
        assert!(
            d <= TABLE_HALF_UNIT,
            "GPU: table pair {i} gives {:?}, not {want:?} (off by {d:e})",
            out[i]
        );
    }
    let n = O_TABLE.len();
    assert!(
        max_abs_diff(out[n], [1.0, 0.0, 0.0]) <= TABLE_HALF_UNIT,
        "GPU: white → {:?}",
        out[n]
    );
    assert!(
        max_abs_diff(out[n + 1], [1.0; 3]) <= TABLE_HALF_UNIT,
        "GPU: (1, 0, 0) → {:?}",
        out[n + 1]
    );
    assert!(
        out[n + 2] == [0.0; 3] && out[n + 3] == [0.0; 3],
        "GPU: black → {:?}, {:?}",
        out[n + 2],
        out[n + 3]
    );
}

#[test]
fn qa_m7_02_wgsl_on_gpu() {
    check_gpu(("", ""));
}

negative_control!(
    qa_m7_02_wgsl_on_gpu,
    "the WGSL's M₁[0][0] wrong in its fourth decimal must break the GPU round trip",
    expected = "GPU: the round trip exceeds",
    check_gpu(("0.4122214708", "0.4123214708"))
);
