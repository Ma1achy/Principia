//! dd_colouring §5's unit test 1 and the transcription check of §3.1 (R-51; REQ-COL-033, REQ-COL-049): the colour
//! spaces' coefficients against Ottosson's reference implementation, the anchors, the polar form, the round trip
//! across the gamut lattice, and the WGSL against the Rust on the GPU.
//!
//! **The reference** (R-184): Björn Ottosson, "A perceptual color space for image processing",
//! <https://bottosson.github.io/posts/oklab/>, at its source, `bottosson/bottosson.github.io`, file
//! `posts/oklab/index.html`, commit `7561fbab5c8b982020ed212aebb0b8620c44b228` (2025-05-26). [`Ottosson`]'s values
//! are transcribed from that revision: the code listing "Converting from linear sRGB to Oklab" (`linear_srgb_to_oklab`
//! and `oklab_to_linear_srgb`), the XYZ matrix `M₁` of "Converting from XYZ to Oklab", and its "Table of example XYZ
//! and Oklab pairs", computed by its author "by transforming the XYZ coordinates to Oklab and rounding to three
//! decimals". The sRGB transfer is not Ottosson's: its constants are checked against dd_colouring §3.1 alone.
//!
//! Three sources must each equal Ottosson's digits, string for string: dd_colouring §3.1 (the corpus R-51 checks),
//! `render::colour::space::OKLAB` (the Rust source) and the WGSL's literals (the prelude's `oklab_to_linear` and
//! `colour_space.wgsl`'s `linear_to_oklab`). On the GPU each WGSL map is compared with the f64 Rust on the same f32
//! input, within the bound WGSL's stated accuracies give (WGSL § "Floating Point Accuracy"), derived at each bound.
//!
//! The round trips use the gate's lattice and threshold (`fixtures/gates/oklab-roundtrip/`): REQ-COL-049's proposed
//! tolerance, provisional until the M7 gate confirms it (R-71, R-182). Each check takes its subject as an argument, so
//! its control runs it on a wrong one and shows it fails (R-176; pitfalls §9).

use std::f64::consts::{FRAC_PI_2, LN_2, PI};
use std::path::Path;

use ledger::gen::prelude;
use render::colour::space::{self, Coefficients, Mat3, OKLAB};
use validation::gpu::{BindingKind, GpuHarness};
use validation::negative_control;
use validation::oklab;

/// Ottosson's published values, as the pinned revision prints them (`+` dropped, `−` written `-`).
struct Ottosson;

impl Ottosson {
    /// `linear_srgb_to_oklab`'s first matrix, linear sRGB → LMS.
    const M1: [[&'static str; 3]; 3] = [
        ["0.4122214708", "0.5363325363", "0.0514459929"],
        ["0.2119034982", "0.6806995451", "0.1073969566"],
        ["0.0883024619", "0.2817188376", "0.6299787005"],
    ];
    /// `linear_srgb_to_oklab`'s second matrix, `lms^{1/3}` → OKLab; also the post's `M₂`.
    const M2: [[&'static str; 3]; 3] = [
        ["0.2104542553", "0.7936177850", "-0.0040720468"],
        ["1.9779984951", "-2.4285922050", "0.4505937099"],
        ["0.0259040371", "0.7827717662", "-0.8086757660"],
    ];
    /// `oklab_to_linear_srgb`'s closed `M₂⁻¹`: the coefficients of `a` and `b` in `l'`, `m'` and `s'` (that of `L`
    /// is 1).
    const M2_INV: [[&'static str; 2]; 3] = [
        ["0.3963377774", "0.2158037573"],
        ["-0.1055613458", "-0.0638541728"],
        ["-0.0894841775", "-1.2914855480"],
    ];
    /// `oklab_to_linear_srgb`'s last matrix, LMS → linear sRGB.
    const M1_INV: [[&'static str; 3]; 3] = [
        ["4.0767416621", "-3.3077115913", "0.2309699292"],
        ["-1.2684380046", "2.6097574011", "-0.3413193965"],
        ["-0.0041960863", "-0.7034186147", "1.7076147010"],
    ];
    /// "Converting from XYZ to Oklab"'s `M₁`, XYZ (D65, white at Y = 1) → LMS.
    const M1_XYZ: [[&'static str; 3]; 3] = [
        ["0.8189330101", "0.3618667424", "-0.1288597137"],
        ["0.0329845436", "0.9293118715", "0.0361456387"],
        ["0.0482003018", "0.2643662691", "0.6338517070"],
    ];
    /// "Table of example XYZ and Oklab pairs": D65 white, then the X, Y and Z primaries; OKLab to three decimals.
    const TABLE: [([&'static str; 3], [&'static str; 3]); 4] = [
        (["0.950", "1.000", "1.089"], ["1.000", "0.000", "0.000"]),
        (["1.000", "0.000", "0.000"], ["0.450", "1.236", "-0.019"]),
        (["0.000", "1.000", "0.000"], ["0.922", "-0.671", "0.263"]),
        (["0.000", "0.000", "1.000"], ["0.153", "-1.415", "-0.449"]),
    ];
}

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn read(rel: &str) -> String {
    let path = root().join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn num(s: &str) -> f64 {
    s.parse()
        .unwrap_or_else(|e| panic!("`{s}` is not a number: {e}"))
}

fn mat(m: &[[&str; 3]; 3]) -> Mat3 {
    m.map(|row| row.map(num))
}

fn mul(m: &Mat3, v: [f64; 3]) -> [f64; 3] {
    m.map(|r| r[0] * v[0] + r[1] * v[1] + r[2] * v[2])
}

/// The f32 unit roundoff, `2⁻²⁴`.
const U: f64 = f32::EPSILON as f64 / 2.0;

// ── The coefficients (REQ-COL-033, R-51) ─────────────────────────────────────────────────────────────────────────

/// `M₂⁻¹` with its first column, 1, from Ottosson's closed coefficients.
fn ottosson_m2_inv() -> Mat3 {
    Ottosson::M2_INV.map(|[a, b]| [1.0, num(a), num(b)])
}

/// Every coefficient of `k` is Ottosson's, exactly: the decimal he prints, read as f64.
fn check_shared_source(k: &Coefficients) {
    for (name, got, want) in [
        ("M₁", k.m1, mat(&Ottosson::M1)),
        ("M₂", k.m2, mat(&Ottosson::M2)),
        ("M₂⁻¹", k.m2_inv, ottosson_m2_inv()),
        ("M₁⁻¹", k.m1_inv, mat(&Ottosson::M1_INV)),
    ] {
        for (i, (g, w)) in got.iter().zip(want).enumerate() {
            assert!(
                *g == w,
                "render::colour::space's {name} row {i} is {g:?}, not Ottosson's {w:?}"
            );
        }
    }
}

#[test]
fn oklab_transcription_shared_source() {
    check_shared_source(&OKLAB);
}

negative_control!(
    oklab_transcription_shared_source,
    "a tenth-decimal slip in M₁ must miss Ottosson's digits",
    expected = "not Ottosson's",
    check_shared_source(&Coefficients {
        m1: [
            [0.4122214709, 0.5363325363, 0.0514459929],
            OKLAB.m1[1],
            OKLAB.m1[2]
        ],
        ..OKLAB
    })
);

/// dd_colouring §3.1, the text between its heading and §3.2's.
fn section_3_1() -> String {
    let doc = read("docs/design/principia_dd_colouring.md");
    let start = doc
        .find("### 3.1 Colour spaces")
        .expect("dd_colouring has §3.1");
    let end = doc[start..].find("### 3.2").expect("dd_colouring has §3.2") + start;
    doc[start..end].to_owned()
}

/// `text` with the minus sign written `-` and `+` dropped, as [`Ottosson`] writes his digits.
fn plain(text: &str) -> String {
    text.replace('−', "-").replace('+', "")
}

/// The nine entries of the matrix `name = [ … ]` in `section`, row by row, as written.
fn doc_matrix(section: &str, name: &str) -> Vec<String> {
    let marker = format!("{name} = [");
    let at = section
        .find(&marker)
        .unwrap_or_else(|| panic!("§3.1 has no `{marker}`"))
        + marker.len();
    let end = section[at..].find(']').expect("the matrix closes") + at;
    plain(&section[at..end])
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

/// The closed coefficients of `a` and `b` in `§3.1`'s `<lms>' = L ± …a ± …b`, signed.
fn doc_closed(section: &str, lms: char) -> Vec<String> {
    let marker = format!("`{lms}' = L ");
    let at = section
        .find(&marker)
        .unwrap_or_else(|| panic!("§3.1 has no `{marker}`"))
        + marker.len();
    let end = section[at..].find('`').expect("the formula closes") + at;
    let text = section[at..end].replace('−', "-");
    let tokens: Vec<&str> = text.split_whitespace().collect();
    tokens
        .chunks(2)
        .map(|pair| {
            let digits = pair[1].trim_end_matches(['a', 'b']);
            if pair[0] == "-" {
                format!("-{digits}")
            } else {
                digits.to_owned()
            }
        })
        .collect()
}

fn flat<const N: usize>(m: &[[&str; N]; 3]) -> Vec<String> {
    m.iter().flatten().map(|s| (*s).to_owned()).collect()
}

/// Every coefficient of dd_colouring §3.1, `section`, is Ottosson's digit for digit (R-51).
fn check_dd_colouring(section: &str) {
    let mut closed = Vec::new();
    for lms in ['l', 'm', 's'] {
        closed.extend(doc_closed(section, lms));
    }
    for (name, got, want) in [
        ("M₁", doc_matrix(section, "M₁"), flat(&Ottosson::M1)),
        ("M₂", doc_matrix(section, "M₂"), flat(&Ottosson::M2)),
        ("M₁⁻¹", doc_matrix(section, "M₁⁻¹"), flat(&Ottosson::M1_INV)),
        ("the closed M₂⁻¹", closed, flat(&Ottosson::M2_INV)),
    ] {
        assert!(
            got == want,
            "dd_colouring §3.1's {name} is {got:?}, not Ottosson's {want:?}"
        );
    }
}

#[test]
fn oklab_transcription_dd_colouring() {
    check_dd_colouring(&section_3_1());
}

negative_control!(
    oklab_transcription_dd_colouring,
    "a transposed pair of digits in §3.1's M₁⁻¹ must miss Ottosson's",
    expected = "not Ottosson's",
    check_dd_colouring(&section_3_1().replace("2.6097574011", "2.6097574101"))
);

/// The sRGB transfer's constants in §3.1's sentence, in order: the slope, the decode's threshold, the offset, the
/// scale, the exponent and the encode's threshold.
fn doc_transfer(section: &str) -> Vec<f64> {
    let line = section
        .lines()
        .find(|l| l.starts_with("**sRGB transfer**"))
        .expect("§3.1 states the sRGB transfer");
    line.split(|c: char| !(c.is_ascii_digit() || c == '.'))
        .filter(|t| t.contains('.') && t.chars().any(|c| c.is_ascii_digit()))
        .map(num)
        .collect()
}

/// `render::colour::space`'s transfer constants are §3.1's, `section`'s, and the f64 transfer at its thresholds is
/// the formula there: the linear segment at and below each, the power segment above.
fn check_transfer(section: &str) {
    let want = doc_transfer(section);
    let got = vec![
        space::SRGB_SLOPE,
        space::SRGB_DECODE_THRESHOLD,
        space::SRGB_OFFSET,
        space::SRGB_SCALE,
        space::SRGB_GAMMA,
        space::SRGB_ENCODE_THRESHOLD,
    ];
    assert!(
        got == want,
        "the transfer's constants are {got:?}, not dd_colouring §3.1's {want:?}"
    );
    let [slope, dec, off, scale, gamma, enc] =
        [want[0], want[1], want[2], want[3], want[4], want[5]];
    let above = 0.5;
    for (what, got, want) in [
        (
            "decode at its threshold",
            space::srgb_to_linear(dec),
            dec / slope,
        ),
        (
            "decode above it",
            space::srgb_to_linear(above),
            ((above + off) / scale).powf(gamma),
        ),
        (
            "encode at its threshold",
            space::linear_to_srgb(enc),
            slope * enc,
        ),
        (
            "encode above it",
            space::linear_to_srgb(above),
            scale * above.powf(1.0 / gamma) - off,
        ),
    ] {
        assert!(
            got == want,
            "the transfer's {what} is {got}, not §3.1's {want}"
        );
    }
}

#[test]
fn oklab_transcription_srgb_transfer() {
    check_transfer(&section_3_1());
}

negative_control!(
    oklab_transcription_srgb_transfer,
    "a §3.1 with the decode's threshold mistyped must miss the constants",
    expected = "not dd_colouring §3.1's",
    check_transfer(&section_3_1().replace("0.04045", "0.04054"))
);

/// The float literals of the WGSL function `name` in `source`, in order, each signed by the `-` before it.
fn wgsl_literals(source: &str, name: &str) -> Vec<String> {
    let marker = format!("fn {name}(");
    let at = source
        .find(&marker)
        .unwrap_or_else(|| panic!("the WGSL has no `{marker}`"));
    let end = source[at..].find("\n}\n").expect("the function closes") + at;
    let body = source[at..end].replace(['(', ')', ',', ';'], " ");
    let tokens: Vec<&str> = body.split_whitespace().collect();
    let mut out = Vec::new();
    for (i, t) in tokens.iter().enumerate() {
        let digits = t.trim_start_matches('-');
        let literal =
            digits.contains('.') && digits.chars().all(|c| c.is_ascii_digit() || c == '.');
        if literal {
            let negative = t.starts_with('-') || (i > 0 && tokens[i - 1] == "-");
            out.push(if negative {
                format!("-{digits}")
            } else {
                digits.to_owned()
            });
        }
    }
    out
}

/// The WGSL's coefficients are Ottosson's digit for digit: `colour_space`'s `linear_to_oklab` is `M₁` then `M₂`, and
/// `prelude`'s `oklab_to_linear` the closed `M₂⁻¹` then `M₁⁻¹`.
fn check_wgsl(colour_space: &str, prelude: &str) {
    let forward = [flat(&Ottosson::M1), flat(&Ottosson::M2)].concat();
    let inverse = [flat(&Ottosson::M2_INV), flat(&Ottosson::M1_INV)].concat();
    for (what, got, want) in [
        (
            "colour_space.wgsl's linear_to_oklab",
            wgsl_literals(colour_space, "linear_to_oklab"),
            forward,
        ),
        (
            "the prelude's oklab_to_linear",
            wgsl_literals(prelude, "oklab_to_linear"),
            inverse,
        ),
    ] {
        assert!(
            got == want,
            "{what}'s coefficients are {got:?}, not Ottosson's {want:?}"
        );
    }
}

const COLOUR_SPACE_WGSL: &str = "crates/render/shaders/wgsl/lib/colour_space.wgsl";

#[test]
fn oklab_transcription_wgsl() {
    check_wgsl(&read(COLOUR_SPACE_WGSL), &read(prelude::PATH));
}

negative_control!(
    oklab_transcription_wgsl,
    "a WGSL M₂ entry with its sign flipped must miss Ottosson's",
    expected = "not Ottosson's",
    check_wgsl(
        &read(COLOUR_SPACE_WGSL).replace("- 0.0040720468", "+ 0.0040720468"),
        &read(prelude::PATH)
    )
);

// ── The anchors (dd_colouring §5, unit test 1) ───────────────────────────────────────────────────────────────────

/// `x` to three decimals, as an integer number of thousandths: Ottosson's table's rounding.
fn thousandths(x: f64) -> i64 {
    (x * 1000.0).round() as i64
}

/// The anchors through `to_lab`, linear sRGB → OKLab: white, `(1, 1, 1)`, goes to `(1, 0, 0)` (dd_colouring §5), in
/// f64 through `to_lab` and in f32 through the shared source; and each of Ottosson's published pairs, its XYZ taken
/// to linear sRGB through his XYZ `M₁` and the shared `M₁⁻¹` (`rgb = M₁⁻¹ · M₁_XYZ · xyz`, Ottosson's two `M₁` both
/// mapping into one LMS), rounds to his OKLab at three decimals. The shared path is then `M₂ · (M₁ · M₁⁻¹ · lms)^{1/3}`,
/// so it checks `M₂`, the real cube root (Z's `l` is negative) and that `M₁` inverts `M₁⁻¹`.
fn check_anchors(to_lab: fn([f64; 3]) -> [f64; 3]) {
    let one = [1000, 0, 0];
    let white = to_lab([1.0; 3]);
    let white32 = space::linear_to_oklab([1.0f32; 3]).map(f64::from);
    for (what, lab) in [("f64", white), ("f32", white32)] {
        assert!(
            lab.map(thousandths) == one,
            "linear sRGB white goes to {lab:?} in {what}, not (1, 0, 0)"
        );
    }
    let m1_xyz = mat(&Ottosson::M1_XYZ);
    for (xyz, want) in Ottosson::TABLE {
        let rgb = mul(&OKLAB.m1_inv, mul(&m1_xyz, xyz.map(num)));
        let got = to_lab(rgb);
        assert!(
            got.map(thousandths) == want.map(|s| thousandths(num(s))),
            "XYZ {xyz:?} goes to OKLab {got:?}, not Ottosson's published {want:?}"
        );
    }
}

#[test]
fn oklab_transcription_anchors() {
    check_anchors(space::linear_to_oklab);
}

negative_control!(
    oklab_transcription_anchors,
    "M₂'s a-row with two third-decimal slips that cancel at white must miss a published pair",
    expected = "not Ottosson's published",
    check_anchors(|rgb| {
        let mut k = OKLAB;
        k.m2[1][0] += 5e-3;
        k.m2[1][1] -= 5e-3;
        space::linear_to_oklab_with(&k, rgb)
    })
);

// ── The polar form (dd_colouring §3.1) ───────────────────────────────────────────────────────────────────────────

/// OKLCH through `to_lch` and back through `to_lab`, in f64: `C = √(a² + b²)` and `h = atan2(b, a)` at the four
/// axis directions and at the 3–4–5 triangle, `h = 0` at the achromatic `C = 0`, signed zeros included (atan2 gives
/// −π at `(−0, −0)`); then `to_lab` inverts `to_lch` on the
/// lattice's colours within `8ε` of the larger of `|L|` and `C` (a handful of f64 roundings).
fn check_polar(to_lch: fn([f64; 3]) -> [f64; 3], to_lab: fn([f64; 3]) -> [f64; 3]) {
    let tight = 4.0 * f64::EPSILON;
    for (lab, want) in [
        ([0.7, 0.1, 0.0], [0.7, 0.1, 0.0]),
        ([0.7, 0.0, 0.1], [0.7, 0.1, FRAC_PI_2]),
        ([0.7, -0.1, 0.0], [0.7, 0.1, PI]),
        ([0.7, 0.0, -0.1], [0.7, 0.1, -FRAC_PI_2]),
        ([0.7, 0.0, 0.0], [0.7, 0.0, 0.0]),
        ([0.7, -0.0, -0.0], [0.7, 0.0, 0.0]),
        ([0.7, 0.03, 0.04], [0.7, 0.05, (4.0f64 / 3.0).atan()]),
    ] {
        let got = to_lch(lab);
        assert!(
            (0..3).all(|i| (got[i] - want[i]).abs() <= tight * want[i].abs().max(1.0)),
            "OKLab {lab:?} goes to OKLCH {got:?}, not {want:?}"
        );
    }
    for lab in lattice_labs() {
        let back = to_lab(to_lch(lab));
        let scale = lab[0].abs().max(lab[1].hypot(lab[2]));
        assert!(
            (0..3).all(|i| (back[i] - lab[i]).abs() <= 8.0 * f64::EPSILON * scale),
            "OKLCH does not invert at OKLab {lab:?}: back at {back:?}"
        );
    }
}

/// The lattice's colours (the gate's, `lattice.json`), in OKLab, f64.
fn lattice_labs() -> Vec<[f64; 3]> {
    lattice()
        .iter()
        .map(|&c| space::srgb_to_oklab(c.map(|x| f64::from(x) / 255.0)))
        .collect()
}

#[test]
fn oklab_transcription_polar() {
    check_polar(space::oklab_to_oklch, space::oklch_to_oklab);
}

negative_control!(
    oklab_transcription_polar,
    "atan2 with its arguments swapped must miss the hue",
    expected = "goes to OKLCH",
    check_polar(
        |[l, a, b]| space::oklab_to_oklch([l, b, a]),
        space::oklch_to_oklab
    )
);

/// The polar form in f32, as the CPU-side generators run it: `to_lch` at [`check_polar`]'s points within `4ε_f32` of
/// the f64 values (relative, or absolute below 1), and `to_lab` inverting it on the lattice's f32 colours within
/// `8ε_f32` of the larger of `|L|` and `C`.
fn check_polar_f32(to_lch: fn([f32; 3]) -> [f32; 3], to_lab: fn([f32; 3]) -> [f32; 3]) {
    let eps = f64::from(f32::EPSILON);
    for (lab, want) in [
        ([0.7f32, 0.1, 0.0], [0.7, 0.1, 0.0]),
        ([0.7, 0.0, 0.1], [0.7, 0.1, FRAC_PI_2]),
        ([0.7, -0.1, 0.0], [0.7, 0.1, PI]),
        ([0.7, 0.0, -0.1], [0.7, 0.1, -FRAC_PI_2]),
        ([0.7, -0.0, -0.0], [0.7, 0.0, 0.0]),
        ([0.7, 0.03, 0.04], [0.7, 0.05, (4.0f64 / 3.0).atan()]),
    ] {
        let got = wide(to_lch(lab));
        assert!(
            (0..3).all(|i| (got[i] - want[i]).abs() <= 4.0 * eps * want[i].abs().max(1.0)),
            "f32 OKLab {lab:?} goes to OKLCH {got:?}, not {want:?}"
        );
    }
    for code in lattice() {
        let lab = space::srgb_to_oklab(code.map(|x| f32::from(x) / 255.0));
        let back = to_lab(to_lch(lab));
        let scale = f64::from(lab[0].abs().max(lab[1].hypot(lab[2])));
        assert!(
            (0..3).all(|i| (f64::from(back[i]) - f64::from(lab[i])).abs() <= 8.0 * eps * scale),
            "f32 OKLCH does not invert at OKLab {lab:?}: back at {back:?}"
        );
    }
}

#[test]
fn oklab_transcription_polar_f32() {
    check_polar_f32(space::oklab_to_oklch, space::oklch_to_oklab);
}

negative_control!(
    oklab_transcription_polar_f32,
    "an f32 inverse taking the hue in turns must miss the colour",
    expected = "f32 OKLCH does not invert",
    check_polar_f32(space::oklab_to_oklch, |[l, c, h]| space::oklch_to_oklab([
        l,
        c,
        h / std::f32::consts::TAU
    ]))
);

/// The composite maps are their definitions, bit for bit, in f32 and f64 at every lattice colour: `srgb_to_oklab`
/// decodes then takes linear sRGB to OKLab, and `oklab_to_srgb` takes OKLab to linear sRGB then encodes.
fn check_compositions(
    srgb_to_oklab: fn([f32; 3]) -> [f32; 3],
    oklab_to_srgb: fn([f32; 3]) -> [f32; 3],
    srgb_to_oklab64: fn([f64; 3]) -> [f64; 3],
    oklab_to_srgb64: fn([f64; 3]) -> [f64; 3],
) {
    for code in lattice() {
        let c = code.map(|x| f32::from(x) / 255.0);
        let lab = space::linear_to_oklab(c.map(space::srgb_to_linear));
        let c64 = wide(c);
        let lab64 = space::linear_to_oklab(c64.map(space::srgb_to_linear));
        for (what, ok) in [
            ("f32 srgb_to_oklab", srgb_to_oklab(c) == lab),
            (
                "f32 oklab_to_srgb",
                oklab_to_srgb(lab) == space::oklab_to_linear(lab).map(space::linear_to_srgb),
            ),
            ("f64 srgb_to_oklab", srgb_to_oklab64(c64) == lab64),
            (
                "f64 oklab_to_srgb",
                oklab_to_srgb64(lab64) == space::oklab_to_linear(lab64).map(space::linear_to_srgb),
            ),
        ] {
            assert!(ok, "{what} is not its definition at sRGB code {code:?}");
        }
    }
}

#[test]
fn oklab_transcription_compositions() {
    check_compositions(
        space::srgb_to_oklab,
        space::oklab_to_srgb,
        space::srgb_to_oklab,
        space::oklab_to_srgb,
    );
}

negative_control!(
    oklab_transcription_compositions,
    "an srgb_to_oklab that skips the decode must miss its definition",
    expected = "f32 srgb_to_oklab is not its definition",
    check_compositions(
        space::linear_to_oklab,
        space::oklab_to_srgb,
        space::srgb_to_oklab,
        space::oklab_to_srgb,
    )
);

// ── The round trip across the gamut lattice (REQ-COL-049) ────────────────────────────────────────────────────────

/// The gate's lattice, `fixtures/gates/oklab-roundtrip/lattice.json`, as 8-bit sRGB codes, red slowest.
fn lattice() -> Vec<[u8; 3]> {
    let input = oklab::input(root(), "lattice.json").unwrap_or_else(|e| panic!("{e}"));
    let codes = oklab::codes(input.stride).unwrap_or_else(|e| panic!("{e}"));
    let mut out = Vec::new();
    for &r in &codes {
        for &g in &codes {
            for &b in &codes {
                out.push([r, g, b]);
            }
        }
    }
    out
}

/// The gate's threshold, REQ-COL-049's proposed tolerance (`fixtures/gates/oklab-roundtrip/gate.json`).
fn tolerance() -> f64 {
    validation::gate::config(root(), oklab::NAME)
        .unwrap_or_else(|e| panic!("{e}"))
        .threshold
        .value
}

/// The round trip through `k` across the lattice, in f32 (the gate's measurement) and in f64, within `tol` on every
/// channel, both linear sRGB ↔ OKLab and the whole chain, encoded.
fn check_round_trip_cpu(k: &Coefficients, tol: f64) {
    for code in lattice() {
        let (lin, enc) = oklab::round_trip(k, code);
        let c = code.map(|x| f64::from(x) / 255.0);
        let l64 = c.map(space::srgb_to_linear);
        let back = space::oklab_to_linear_with(k, space::linear_to_oklab_with(k, l64));
        let lin64 = (0..3).map(|i| (back[i] - l64[i]).abs()).fold(0.0, f64::max);
        let enc64 = (0..3)
            .map(|i| (space::linear_to_srgb(back[i]) - c[i]).abs())
            .fold(0.0, f64::max);
        for (what, e) in [
            ("f32 linear", lin),
            ("f32 encoded", enc),
            ("f64 linear", lin64),
            ("f64 encoded", enc64),
        ] {
            assert!(
                e <= tol,
                "the {what} round trip at sRGB code {code:?} is off by {e:e}, beyond the tolerance {tol:e}"
            );
        }
    }
}

#[test]
fn oklab_transcription_round_trip_cpu() {
    check_round_trip_cpu(&OKLAB, tolerance());
}

negative_control!(
    oklab_transcription_round_trip_cpu,
    "M₁⁻¹ with a fourth-decimal slip must break the round trip",
    expected = "beyond the tolerance",
    check_round_trip_cpu(
        &Coefficients {
            m1_inv: [
                [4.0768416621, -3.3077115913, 0.2309699292],
                OKLAB.m1_inv[1],
                OKLAB.m1_inv[2]
            ],
            ..OKLAB
        },
        tolerance()
    )
);

// ── The WGSL on the GPU (REQ-COL-033) ────────────────────────────────────────────────────────────────────────────

/// The test entry appended to the prelude and `colour_space.wgsl`: each pixel of a `CASE_WIDTH`-wide target applies
/// one operation to one colour, `cases[i]` for the pixel at index i (row-major; the padding pixels repeat the last
/// case), and returns the result's bits. Word 0 is the operation, words 1–3 the colour's bits. Operations 7 and 8 are
/// the round trips, returning the difference from the start: the encoded chain, and linear ↔ OKLab from the decoded
/// colour.
const ENTRY: &str = r"
@group(2) @binding(0) var<storage, read> cases: array<vec4<u32>>;

const CASE_WIDTH: u32 = 1024u;

@fragment
fn t_colour_space(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<u32> {
    let i = min(u32(pos.y) * CASE_WIDTH + u32(pos.x), arrayLength(&cases) - 1u);
    let c = cases[i];
    let x = bitcast<vec3<f32>>(c.yzw);
    var r = vec3<f32>(0.0);
    switch c.x {
        case 1u: { r = linear_to_oklab(x); }
        case 2u: { r = oklab_to_linear(x); }
        case 3u: { r = srgb_to_oklab(x); }
        case 4u: { r = oklab_to_srgb(x); }
        case 5u: { r = oklab_to_oklch(x); }
        case 6u: { r = oklch_to_oklab(x); }
        case 7u: { r = oklab_to_srgb(srgb_to_oklab(x)) - x; }
        case 8u: {
            let lin = srgb_to_linear(x);
            r = oklab_to_linear(linear_to_oklab(lin)) - lin;
        }
        default: {}
    }
    return vec4<u32>(bitcast<vec3<u32>>(r), 0u);
}
";

/// `ENTRY`'s target width.
const CASE_WIDTH: usize = 1024;

const LINEAR_TO_OKLAB: u32 = 1;
const OKLAB_TO_LINEAR: u32 = 2;
const SRGB_TO_OKLAB: u32 = 3;
const OKLAB_TO_SRGB: u32 = 4;
const OKLAB_TO_OKLCH: u32 = 5;
const OKLCH_TO_OKLAB: u32 = 6;
const ROUND_TRIP_SRGB: u32 = 7;
const ROUND_TRIP_LINEAR: u32 = 8;

/// The WGSL the GPU checks read, with at most one mutation, a control's: `from` replaced by `to`.
#[derive(Clone, Copy, Debug)]
struct Text {
    from: &'static str,
    to: &'static str,
}

const AS_IS: Text = Text { from: "", to: "" };

impl Text {
    fn apply(self, text: String) -> String {
        if self.from.is_empty() {
            return text;
        }
        assert!(
            text.contains(self.from),
            "the mutation's target `{}` is not in the WGSL",
            self.from
        );
        text.replace(self.from, self.to)
    }
}

/// Each `(op, colour)` evaluated on the GPU after the checked-in prelude and `colour_space.wgsl`, `text` applied:
/// one draw, a pixel per case, the result as f32.
fn run(text: Text, cases: &[(u32, [f32; 3])]) -> Vec<[f32; 3]> {
    let gpu = GpuHarness::new().expect("a GPU device");
    let module = text.apply(format!(
        "{}\n{}\n{ENTRY}",
        read(prelude::PATH),
        read(COLOUR_SPACE_WGSL)
    ));
    let words: Vec<u32> = cases
        .iter()
        .flat_map(|(op, x)| [*op, x[0].to_bits(), x[1].to_bits(), x[2].to_bits()])
        .collect();
    let rows = cases.len().div_ceil(CASE_WIDTH);
    let kernel = gpu
        .fragment(
            &module,
            "t_colour_space",
            &[&[], &[], &[BindingKind::Storage]],
            CASE_WIDTH as u32,
            rows as u32,
        )
        .unwrap_or_else(|e| panic!("{e}"));
    let pixels = kernel
        .draw(&[&[], &[], &[&words]])
        .unwrap_or_else(|e| panic!("{e}"));
    pixels[..cases.len()]
        .iter()
        .map(|p| [p[0], p[1], p[2]].map(f32::from_bits))
        .collect()
}

/// WGSL's absolute error bound on `log2(v)`: `2⁻²¹` on [0.5, 2], else 3 ULP, at most `6U·|log2 v|`.
fn log2_err(v: f64) -> f64 {
    if (0.5..=2.0).contains(&v) {
        2f64.powi(-21)
    } else {
        6.0 * U * v.log2().abs()
    }
}

/// The relative error bound of WGSL's `pow(v, y)`, `v > 0`, for an exponent `y` that is f32 rounding of the exact
/// one: `pow` is inherited from `exp2(y · log2 v)`. The exponent `t = y·log2 v` is off by `|y|·log2_err(v)` plus its
/// product's rounding, `U·|t|`, which `exp2` turns into `ln 2` times as much relatively; `exp2` itself is within
/// `3 + 2|t|` ULP, at most `(6 + 4|t|)·U` relatively; and `y`'s own rounding, relative `U`, moves the result by
/// `|y·ln v|·U` relatively.
fn pow_rel(v: f64, y: f64) -> f64 {
    let t = y * v.log2();
    LN_2 * (y.abs() * log2_err(v) + U * t.abs())
        + (6.0 + 4.0 * t.abs()) * U
        + (y * v.ln()).abs() * U
}

/// The bound on a 3-term f32 dot product with f32-rounded coefficients `row` and exact `x`: four roundings per term
/// (the coefficient, the product, two sums), `4.01U` times the sum of the terms' magnitudes.
fn dot_bound(row: &[f64; 3], x: [f64; 3]) -> f64 {
    4.01 * U * (0..3).map(|j| (row[j] * x[j]).abs()).sum::<f64>()
}

/// The decoded value's relative error bound on the GPU, `srgb_to_linear(c)`: on the linear segment, `c / 12.92`, a
/// division (2.5 ULP, `5U`) and the constant's rounding (`U`); on the power segment, `x = (c + 0.055)/1.055` within
/// `8U` (the sum, both constants, the division), which `x^2.4` multiplies by 2.4, plus `pow`'s own ([`pow_rel`]).
fn decode_rel(c: f64) -> f64 {
    if c <= space::SRGB_DECODE_THRESHOLD {
        6.0 * U
    } else {
        let x = (c + space::SRGB_OFFSET) / space::SRGB_SCALE;
        space::SRGB_GAMMA * 8.0 * U + pow_rel(x, space::SRGB_GAMMA)
    }
}

/// Each OKLab channel's bound on the GPU's `linear_to_oklab(rgb)` against the f64 one, each `rgb` channel already off
/// by `rel_in` of itself. `lms` is off by its dot product's rounding ([`dot_bound`]) and the input's error through
/// `M₁`. The cube root of `v = |lms|` moves by at most `(v − e)^{−2/3}·e/3` for an input error `e < v/2`, else by
/// `2^{2/3}·e^{1/3}` (the cube root's modulus of continuity), and `pow` adds [`pow_rel`] of the root; a zero `lms` is
/// a zero root, exactly, both sides. `M₂` then sums the roots' errors with its absolute coefficients, plus its own
/// dot product's rounding. Second-order terms are below 1% here: the total is raised by 1%.
fn forward_bound(rgb: [f64; 3], rel_in: [f64; 3]) -> [f64; 3] {
    let roots: Vec<(f64, f64)> = OKLAB
        .m1
        .iter()
        .map(|row| {
            let lms = row[0] * rgb[0] + row[1] * rgb[1] + row[2] * rgb[2];
            let e = dot_bound(row, rgb)
                + (0..3)
                    .map(|j| (row[j] * rgb[j] * rel_in[j]).abs())
                    .sum::<f64>();
            let v = lms.abs();
            let q = v.cbrt();
            let moved = if e < v / 2.0 {
                (v - e).powf(-2.0 / 3.0) * e / 3.0
            } else {
                2f64.powf(2.0 / 3.0) * e.cbrt()
            };
            let own = if v > 0.0 {
                q * pow_rel(v, 1.0 / 3.0)
            } else {
                0.0
            };
            (q.copysign(lms), moved + own)
        })
        .collect();
    OKLAB.m2.map(|row| {
        let q = [roots[0].0, roots[1].0, roots[2].0];
        1.01 * ((0..3).map(|i| row[i].abs() * roots[i].1).sum::<f64>() + dot_bound(&row, q))
    })
}

/// Each linear channel's bound on the GPU's `oklab_to_linear(lab)` against the f64 one, `lab` exact: `p = M₂⁻¹·Lab`
/// within its dot product's rounding, `e_p`; the cube `p³` within `3p²·e_p` plus two products' rounding, `2.01U·|p|³`;
/// `M₁⁻¹` sums those with its absolute coefficients, plus its own dot product's rounding; raised by 1% for the
/// second-order terms.
fn inverse_bound(lab: [f64; 3]) -> [f64; 3] {
    let cubes: Vec<(f64, f64)> = OKLAB
        .m2_inv
        .iter()
        .map(|row| {
            let p = row[0] * lab[0] + row[1] * lab[1] + row[2] * lab[2];
            let e = dot_bound(row, lab);
            (p * p * p, 3.0 * p * p * e + 2.01 * U * (p * p * p).abs())
        })
        .collect();
    OKLAB.m1_inv.map(|row| {
        let lms = [cubes[0].0, cubes[1].0, cubes[2].0];
        1.01 * ((0..3).map(|i| row[i].abs() * cubes[i].1).sum::<f64>() + dot_bound(&row, lms))
    })
}

/// The gap between the transfer's two segments at a threshold: a value within an error of it may take either
/// branch, so the bound adds it there.
fn junction(decode: bool) -> f64 {
    if decode {
        let t = space::SRGB_DECODE_THRESHOLD;
        (t / space::SRGB_SLOPE
            - ((t + space::SRGB_OFFSET) / space::SRGB_SCALE).powf(space::SRGB_GAMMA))
        .abs()
    } else {
        let t = space::SRGB_ENCODE_THRESHOLD;
        (space::SRGB_SLOPE * t
            - (space::SRGB_SCALE * t.powf(1.0 / space::SRGB_GAMMA) - space::SRGB_OFFSET))
            .abs()
    }
}

/// The bound on the GPU's `linear_to_srgb` of a linear value `lin` already off by `e`: the encode's slope over
/// `[lin − e, lin + e]` times `e` (12.92 on the linear segment, and above the power segment's slope everywhere on it;
/// the power segment's falls with `lin`), plus the encode's own error: `12.92·c` within `2.01U`, relatively;
/// `1.055·c^{1/2.4} − 0.055` within [`pow_rel`] and two roundings of the power, then the difference's rounding and
/// the offset's; and the segments' gap where `lin` is within `e` of the threshold.
fn encode_bound(lin: f64, e: f64) -> f64 {
    let t = space::SRGB_ENCODE_THRESHOLD;
    let lo = lin - e;
    let slope = if lo <= t {
        space::SRGB_SLOPE
    } else {
        space::SRGB_SCALE / space::SRGB_GAMMA * lo.powf(1.0 / space::SRGB_GAMMA - 1.0)
    };
    let own = if lin <= t {
        2.01 * U * space::SRGB_SLOPE * lin.abs()
    } else {
        let v = space::SRGB_SCALE * lin.powf(1.0 / space::SRGB_GAMMA);
        v * (pow_rel(lin, 1.0 / space::SRGB_GAMMA) + 2.01 * U)
            + U * (v - space::SRGB_OFFSET).abs()
            + U * space::SRGB_OFFSET
    };
    let gap = if (lin - t).abs() <= e {
        junction(false)
    } else {
        0.0
    };
    slope * e + own + gap
}

/// The bound on the GPU's `oklab_to_oklch(lab)` against the f64 one: `L` is passed through exactly; `C`, a square
/// root (inherited from `1/inverseSqrt`: 2 ULP, then 2.5 ULP for the reciprocal, `9U`) of a sum within `3.01U`, within
/// `10.6U` of itself, written `11U`; `h` within atan2's 4096 ULP, at most `8192U·|h|`, where WGSL bounds it: both
/// arguments normal. Elsewhere `h` is unbounded (infinity), and `C = 0` gives `h = 0` exactly on both sides.
fn oklch_bound(lab: [f64; 3]) -> [f64; 3] {
    let [_, a, b] = lab;
    let c = a.hypot(b);
    let normal = |x: f64| x.abs() >= f64::from(f32::MIN_POSITIVE);
    let h = if c == 0.0 {
        0.0
    } else if normal(a) && normal(b) {
        8192.0 * U * b.atan2(a).abs()
    } else {
        f64::INFINITY
    };
    [0.0, 11.0 * U * c, h]
}

/// The bound on the GPU's `oklch_to_oklab(lch)`, `h` in [−π, π]: `L` exact; `a` and `b` within `C·2⁻¹¹`, WGSL's
/// `cos` and `sin` there, plus the product's rounding.
fn polar_bound(lch: [f64; 3]) -> [f64; 3] {
    let c = lch[1].abs();
    [
        0.0,
        c * (2f64.powi(-11) + 2.01 * U),
        c * (2f64.powi(-11) + 2.01 * U),
    ]
}

/// `lch`'s hue moved into [−π, π] if f32 rounding put it just outside (f32's nearest π is above π).
fn in_range(lch: [f32; 3]) -> [f32; 3] {
    let h = lch[2];
    let h = if f64::from(h).abs() > PI {
        f32::from_bits(h.to_bits() - 1)
    } else {
        h
    };
    [lch[0], lch[1], h]
}

fn wide(x: [f32; 3]) -> [f64; 3] {
    x.map(f64::from)
}

/// Every WGSL colour-space map, `text` applied, against the f64 Rust on the same f32 input, at every colour of the
/// lattice: `linear_to_oklab` and `srgb_to_oklab` on the colour (linear and encoded), `oklab_to_linear`,
/// `oklab_to_srgb` and `oklab_to_oklch` on its f32 OKLab, `oklch_to_oklab` on its f32 OKLCH; each channel within the
/// bound derived for it.
fn check_gpu_agrees(text: Text) {
    let mut cases = Vec::new();
    for code in lattice() {
        let c = code.map(|x| f32::from(x) / 255.0);
        let lin = c.map(space::srgb_to_linear);
        let lab = space::linear_to_oklab(lin);
        let lch = in_range(space::oklab_to_oklch(lab));
        cases.extend([
            (LINEAR_TO_OKLAB, lin),
            (SRGB_TO_OKLAB, c),
            (OKLAB_TO_LINEAR, lab),
            (OKLAB_TO_SRGB, lab),
            (OKLAB_TO_OKLCH, lab),
            (OKLCH_TO_OKLAB, lch),
        ]);
    }
    let got = run(text, &cases);
    for ((op, x), g) in cases.iter().zip(&got) {
        let x64 = wide(*x);
        let (name, want, bound) = match *op {
            LINEAR_TO_OKLAB => (
                "linear_to_oklab",
                space::linear_to_oklab(x64),
                forward_bound(x64, [0.0; 3]),
            ),
            SRGB_TO_OKLAB => {
                let lin = x64.map(space::srgb_to_linear);
                let rel = x64.map(|c| {
                    let gap = (c - space::SRGB_DECODE_THRESHOLD).abs() <= 8.0 * U * c;
                    decode_rel(c)
                        + if gap {
                            junction(true) / space::srgb_to_linear(c)
                        } else {
                            0.0
                        }
                });
                (
                    "srgb_to_oklab",
                    space::srgb_to_oklab(x64),
                    forward_bound(lin, rel),
                )
            }
            OKLAB_TO_LINEAR => (
                "oklab_to_linear",
                space::oklab_to_linear(x64),
                inverse_bound(x64),
            ),
            OKLAB_TO_SRGB => {
                let lin = space::oklab_to_linear(x64);
                let e = inverse_bound(x64);
                let bound = [0, 1, 2].map(|i| encode_bound(lin[i], e[i]));
                ("oklab_to_srgb", space::oklab_to_srgb(x64), bound)
            }
            OKLAB_TO_OKLCH => (
                "oklab_to_oklch",
                space::oklab_to_oklch(x64),
                oklch_bound(x64),
            ),
            _ => (
                "oklch_to_oklab",
                space::oklch_to_oklab(x64),
                polar_bound(x64),
            ),
        };
        let g64 = wide(*g);
        assert!(
            (0..3).all(|i| (g64[i] - want[i]).abs() <= bound[i]),
            "the WGSL {name}({x:?}) is {g:?}; the Rust's is {want:?}, beyond the bound {bound:?}"
        );
    }
}

#[test]
fn oklab_transcription_gpu_agrees() {
    check_gpu_agrees(AS_IS);
}

negative_control!(
    oklab_transcription_gpu_agrees,
    "the WGSL's M₂ with an entry's sign flipped must miss the Rust",
    expected = "the WGSL linear_to_oklab",
    check_gpu_agrees(Text {
        from: "0.7936177850 * q.y - 0.0040720468 * q.z",
        to: "0.7936177850 * q.y + 0.0040720468 * q.z",
    })
);

/// The round trip on the GPU across the lattice, `text` applied, within `tol` on every channel: the encoded chain,
/// `oklab_to_srgb(srgb_to_oklab(c))`, and linear sRGB ↔ OKLab from the decoded colour. Prints the largest of each.
fn check_gpu_round_trip(text: Text, tol: f64) {
    let mut cases = Vec::new();
    for code in lattice() {
        let c = code.map(|x| f32::from(x) / 255.0);
        cases.extend([(ROUND_TRIP_SRGB, c), (ROUND_TRIP_LINEAR, c)]);
    }
    let got = run(text, &cases);
    let mut worst = [(0.0, [0.0f32; 3]); 2];
    for ((op, c), g) in cases.iter().zip(&got) {
        let e = g.iter().map(|d| f64::from(d.abs())).fold(0.0, f64::max);
        let (what, k) = if *op == ROUND_TRIP_SRGB {
            ("encoded", 0)
        } else {
            ("linear", 1)
        };
        if e > worst[k].0 {
            worst[k] = (e, *c);
        }
        assert!(
            e <= tol,
            "the GPU's {what} round trip at sRGB {c:?} is off by {e:e}, beyond the tolerance {tol:e}"
        );
    }
    println!(
        "GPU round trip over {} colours: encoded max {:.3e} at sRGB {:?}; linear max {:.3e} at sRGB {:?}",
        cases.len() / 2,
        worst[0].0,
        worst[0].1,
        worst[1].0,
        worst[1].1
    );
}

#[test]
fn oklab_transcription_gpu_round_trip() {
    check_gpu_round_trip(AS_IS, tolerance());
}

negative_control!(
    oklab_transcription_gpu_round_trip,
    "a cube root taken as the 1/3.1 power must break the GPU's round trip",
    expected = "beyond the tolerance",
    check_gpu_round_trip(
        Text {
            from: "vec3<f32>(1.0 / 3.0)",
            to: "vec3<f32>(1.0 / 3.1)",
        },
        tolerance()
    )
);
