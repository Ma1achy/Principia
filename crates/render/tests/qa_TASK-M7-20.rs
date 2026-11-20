//! qa's tests for TASK-M7-20 (REQ-COL-042, REQ-COL-045, REQ-COL-061; R-78, R-123, R-383, R-386; PIT-3), written from
//! the requirements, dd_colouring §3.8 and §5 unit test 10, colour_composition §4.3 and the R-383 record, not from the
//! implementation.
//!
//! **qa's own derivation.** R-383 records the reference's data and construction: Viénot et al.'s matrix to Judd–Vos XYZ
//! (`convert.py:186–190`), Smith & Pokorny 1975's cone fundamentals (`convert.py:160–164`), their product as the LMS
//! model, Viénot's plane through black, blue and yellow with the missing cone's axis projected onto it, and Brettel's
//! two half-planes through white's LMS image and the Judd–Vos XYZ of 485 nm and 660 nm, a colour taking the half-plane
//! of the anchor on its side of the plane through white and the S axis. qa builds every transform here from those
//! numbers alone, with its own inverse (Gauss–Jordan, not an adjugate), and its own side rule (the colour's side
//! against the anchor's, not "the first"), and checks:
//! - the goldens in `fixtures/cvd/goldens.json` (their matrices, their linear values, every simulated colour) are that
//!   construction's, and dd_colouring §3.8's six-decimal transcription is too;
//! - the crate's mirror `render::display::cvd::simulate` agrees with it off the goldens' lattice, near Brettel's
//!   separating plane included;
//! - the method's own properties (Viénot 1999; Brettel 1997): the kept cones are kept, the result lies on the plane or
//!   on the colour's half-plane, white is fixed, and the dichromat image is its own image;
//! - the pass on the GPU, under each of the Display window's five modes by its code, against qa's derivation within
//!   the derived f32 bound; achromatopsia `R = G = B` exactly; off the identity bit for bit;
//! - the pass applied before linearisation is detected under every simulation (unit test 10's "asserting the stage");
//! - REQ-COL-061's proposal: its numbers (the threshold, each stripe's least distance and the stripes' contrast per
//!   mode, R-386's achromatopsia lightnesses) are reproduced by qa's simulation and qa's OKLab, and its verdict, the
//!   pattern distinct under every simulation with a stripe colour inside the threshold under each, holds.
//!
//! The threshold of REQ-COL-061 is not re-measured: the human confirms it at the M1 gate (R-71, R-386).
//!
//! Every check takes its subject as an argument, and its negative control runs it on a wrong one (R-176).

use std::path::Path;

use render::display::cvd::{self, CvdMode, CvdPass};
use validation::gpu::GpuHarness;
use validation::negative_control;

#[path = "support/palettes.rs"]
mod palettes;

type M3 = [[f64; 3]; 3];
type V3 = [f64; 3];

// ── R-383's recorded data, transcribed by qa from decisions.md § R-383 ──────────────────────────────────────────────

/// `XYZJuddVos_from_linearRGB_BT709`, `1e-2 ·` this (R-383).
const QA_JV_PERCENT: M3 = [
    [40.9568, 35.5041, 17.9167],
    [21.3389, 70.6743, 7.98680],
    [1.86297, 11.4620, 91.2367],
];

/// `LMS_from_XYZJuddVos_Smith_Pokorny_1975` (R-383).
const QA_SP: M3 = [
    [0.15514, 0.54312, -0.03286],
    [-0.15514, 0.45684, 0.03286],
    [0.0, 0.0, 0.01608],
];

/// The "widely quoted" product R-383 gives, `0.01 ×` this, to its six significant figures.
const QA_QUOTED_PERCENT: M3 = [
    [17.8824, 43.5161, 4.11935],
    [3.45565, 27.1554, 3.86714],
    [0.0299566, 0.184309, 1.46709],
];

/// Brettel's tritan anchors, the Judd–Vos XYZ of 485 nm and 660 nm (dd_colouring §3.8, `simulate.py:239–240`).
const QA_XYZ_485: V3 = [0.05699, 0.16987, 0.5864];
const QA_XYZ_660: V3 = [0.16161, 0.061, 0.00001];

/// dd_colouring §3.8's `M_achrom` row.
const QA_ACHROM: V3 = [0.299, 0.587, 0.114];

/// f64 and f32 unit roundoffs.
const EPS: f64 = f64::EPSILON;
const U32: f64 = f32::EPSILON as f64 / 2.0;

fn mm(a: &M3, b: &M3) -> M3 {
    let mut r = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            for k in 0..3 {
                r[i][j] += a[i][k] * b[k][j];
            }
        }
    }
    r
}

fn mv(a: &M3, v: V3) -> V3 {
    [0, 1, 2].map(|i| a[i][0] * v[0] + a[i][1] * v[1] + a[i][2] * v[2])
}

fn abs_m(a: &M3) -> M3 {
    a.map(|r| r.map(f64::abs))
}

fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// The inverse by Gauss–Jordan elimination with partial pivoting (a route other than the crate's adjugate).
fn gauss_jordan(m: &M3) -> M3 {
    let mut a = [[0.0; 6]; 3];
    for i in 0..3 {
        a[i][..3].copy_from_slice(&m[i]);
        a[i][3 + i] = 1.0;
    }
    for col in 0..3 {
        let p = (col..3)
            .max_by(|&x, &y| a[x][col].abs().total_cmp(&a[y][col].abs()))
            .unwrap();
        a.swap(col, p);
        let d = a[col][col];
        for x in a[col].iter_mut() {
            *x /= d;
        }
        for r in 0..3 {
            if r != col {
                let (f, pivot) = (a[r][col], a[col]);
                for (x, y) in a[r].iter_mut().zip(pivot) {
                    *x -= f * y;
                }
            }
        }
    }
    std::array::from_fn(|i| std::array::from_fn(|j| a[i][3 + j]))
}

/// The projection along LMS axis `k` onto the plane through black with normal `n`: cone `k`'s response becomes the
/// one that puts the colour on the plane, `x_k = −Σ_{j≠k} n_j x_j / n_k`; the other two are kept.
fn project_along(n: V3, k: usize) -> M3 {
    let mut p = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    p[k] = [0, 1, 2].map(|j| if j == k { 0.0 } else { -n[j] / n[k] });
    p
}

/// qa's construction of every transform from R-383's data.
struct Qa {
    lms: M3,
    inv: M3,
    protan: M3,
    deutan: M3,
    /// Brettel tritan's transform on linear RGB for the half-plane through 660 nm and through 485 nm.
    t660: M3,
    t485: M3,
    /// The separating plane's normal (white × S axis), in LMS, and the 660 nm anchor's side of it.
    n_sep: V3,
    side_660: f64,
    /// The planes' normals, in LMS: Viénot's, Brettel's two.
    n_vienot: V3,
    n_660: V3,
    n_485: V3,
    lms_660: V3,
    lms_485: V3,
}

fn qa() -> Qa {
    let jv = QA_JV_PERCENT.map(|r| r.map(|x| x * 1e-2));
    let lms = mm(&QA_SP, &jv);
    let inv = gauss_jordan(&lms);
    let on_rgb = |p: &M3| mm(&inv, &mm(p, &lms));
    let n_vienot = cross(mv(&lms, [1.0, 1.0, 0.0]), mv(&lms, [0.0, 0.0, 1.0]));
    let white = mv(&lms, [1.0, 1.0, 1.0]);
    let (lms_485, lms_660) = (mv(&QA_SP, QA_XYZ_485), mv(&QA_SP, QA_XYZ_660));
    let (n_485, n_660) = (cross(white, lms_485), cross(white, lms_660));
    let n_sep = cross(white, [0.0, 0.0, 1.0]);
    Qa {
        lms,
        inv,
        protan: on_rgb(&project_along(n_vienot, 0)),
        deutan: on_rgb(&project_along(n_vienot, 1)),
        t660: on_rgb(&project_along(n_660, 2)),
        t485: on_rgb(&project_along(n_485, 2)),
        n_sep,
        side_660: dot(n_sep, lms_660),
        n_vienot,
        n_660,
        n_485,
        lms_660,
        lms_485,
    }
}

impl Qa {
    /// Brettel's choice for the linear colour `c`: the half-plane of the anchor on `c`'s side of the separating plane.
    /// On the plane itself both give the same point, on the neutral axis.
    fn tritan_matrix(&self, c: V3) -> &M3 {
        if dot(self.n_sep, mv(&self.lms, c)) * self.side_660 >= 0.0 {
            &self.t660
        } else {
            &self.t485
        }
    }

    /// qa's simulation of the linear colour `c` (unclamped) under `mode`.
    fn sim(&self, mode: CvdMode, c: V3) -> V3 {
        match mode {
            CvdMode::Off => c,
            CvdMode::Protanopia => mv(&self.protan, c),
            CvdMode::Deuteranopia => mv(&self.deutan, c),
            CvdMode::Tritanopia => mv(self.tritan_matrix(c), c),
            CvdMode::Achromatopsia => [dot(QA_ACHROM, c); 3],
        }
    }

    /// The f64 error bound of `sim` against another f64 route to the same value: `16 ε (|M⁻¹| |P| |M| |c|)ᵢ`, the
    /// rounding of a product of three 3 × 3 matrices and a vector, whichever way it is associated, with room.
    fn bound64(&self, mode: CvdMode, c: V3) -> V3 {
        let p = match mode {
            CvdMode::Protanopia => project_along(self.n_vienot, 0),
            CvdMode::Deuteranopia => project_along(self.n_vienot, 1),
            CvdMode::Tritanopia => {
                if std::ptr::eq(self.tritan_matrix(c), &self.t660) {
                    project_along(self.n_660, 2)
                } else {
                    project_along(self.n_485, 2)
                }
            }
            _ => return [16.0 * EPS * c.iter().map(|x| x.abs()).sum::<f64>(); 3],
        };
        let chain = mm(&abs_m(&self.inv), &mm(&abs_m(&p), &abs_m(&self.lms)));
        mv(&chain, c.map(f64::abs)).map(|x| 16.0 * EPS * x)
    }
}

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn goldens() -> serde_json::Value {
    let path = root().join("fixtures/cvd/goldens.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn v3(v: &serde_json::Value) -> V3 {
    [0, 1, 2].map(|k| v[k].as_f64().expect("a number"))
}

fn m3(v: &serde_json::Value) -> M3 {
    [0, 1, 2].map(|k| v3(&v[k]))
}

/// dd_colouring §3.1's sRGB decode, `c/12.92` at or below 0.04045, else `((c + 0.055)/1.055)^2.4`; R-383 records the
/// reference's as the same constants (it takes `<` at the threshold, which no 8-bit code reaches: 0.04045 · 255 is
/// 10.31).
fn qa_decode(c8: u8) -> f64 {
    let c = f64::from(c8) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// The sRGB encode, for the screen's expected 8-bit value.
fn qa_encode(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    if x <= 0.0031308 {
        12.92 * x
    } else {
        1.055 * x.powf(1.0 / 2.4) - 0.055
    }
}

fn close_m(name: &str, got: &M3, want: &M3, tol: f64) {
    for i in 0..3 {
        for j in 0..3 {
            assert!(
                (got[i][j] - want[i][j]).abs() <= tol,
                "{name}[{i}][{j}] is {}, qa's derivation gives {} (tolerance {tol:e})",
                got[i][j],
                want[i][j]
            );
        }
    }
}

// ── REQ-COL-042: the goldens are R-383's construction ───────────────────────────────────────────────────────────────

/// The R-383 record's own checks of qa's data: the product is 0.01 × the widely quoted matrix to its quoted figures.
#[test]
fn qa_cvd_simulation_r383_lms_is_the_quoted_product() {
    let q = qa();
    let quoted = QA_QUOTED_PERCENT.map(|r| r.map(|x| x * 1e-2));
    // The quoted figures are six significant: half a unit in the sixth is 5e-6 relative of the largest, 0.435.
    close_m("LMS_from_linearRGB", &q.lms, &quoted, 5e-6 * 0.44);
}

negative_control!(
    qa_cvd_simulation_r383_lms_is_the_quoted_product,
    "the LMS model without Smith–Pokorny's L row sign (a transcription slip) must not give the quoted product",
    expected = "LMS_from_linearRGB[",
    {
        let mut sp = QA_SP;
        sp[1][0] = 0.15514;
        let jv = QA_JV_PERCENT.map(|r| r.map(|x| x * 1e-2));
        let quoted = QA_QUOTED_PERCENT.map(|r| r.map(|x| x * 1e-2));
        close_m("LMS_from_linearRGB", &mm(&sp, &jv), &quoted, 5e-6 * 0.44)
    }
);

/// Every golden matrix, linear value and simulated colour is qa's construction from R-383's data, within the f64
/// bound; the goldens name the pinned commit and the linear path; the test colour set holds the hatch's two colours.
fn check_goldens(q: &Qa) {
    let g = goldens();
    assert_eq!(
        g["source"]["commit"].as_str(),
        Some("3cba5e6a7c8f0e8199c8f83f1afb58eb6dab7a3d"),
        "the goldens do not name R-383's commit"
    );
    assert!(
        g["source"]["path"].as_str().is_some_and(
            |p| p.contains("_simulate_dichromacy_linear_rgb") && p.contains("severity 1")
        ),
        "the goldens do not say they are the linear-RGB path at full dichromacy"
    );
    let m = &g["matrices"];
    let tol = 64.0 * EPS * 70.0; // the inverse's entries reach 69.4
    close_m(
        "goldens LMS_from_linearRGB",
        &m3(&m["LMS_from_linearRGB"]),
        &q.lms,
        tol,
    );
    close_m(
        "goldens linearRGB_from_LMS",
        &m3(&m["linearRGB_from_LMS"]),
        &q.inv,
        tol,
    );
    close_m(
        "goldens vienot_protan",
        &m3(&m["vienot_protan"]),
        &q.protan,
        tol,
    );
    close_m(
        "goldens vienot_deutan",
        &m3(&m["vienot_deutan"]),
        &q.deutan,
        tol,
    );
    close_m(
        "goldens brettel T1 (660 nm)",
        &m3(&m["brettel_tritan"]["T1"]),
        &q.t660,
        tol,
    );
    close_m(
        "goldens brettel T2 (485 nm)",
        &m3(&m["brettel_tritan"]["T2"]),
        &q.t485,
        tol,
    );
    let colours = g["colours"].as_array().expect("colours");
    let mut seen_hatch = [false; 2];
    for c in colours {
        let s8 = [0, 1, 2].map(|k| c["srgb8"][k].as_u64().expect("8-bit") as u8);
        seen_hatch[0] |= s8 == [0x9B, 0x00, 0xFF];
        seen_hatch[1] |= s8 == [0x48, 0xFF, 0xFF];
        let lin = s8.map(qa_decode);
        let golden_lin = v3(&c["linear"]);
        assert!(
            (0..3).all(|k| (golden_lin[k] - lin[k]).abs() <= 4.0 * EPS),
            "the golden linear value of {s8:?} is {golden_lin:?}, the sRGB decode gives {lin:?}"
        );
        for (key, mode) in [
            ("protan", CvdMode::Protanopia),
            ("deutan", CvdMode::Deuteranopia),
            ("tritan", CvdMode::Tritanopia),
        ] {
            let (want, got, b) = (q.sim(mode, lin), v3(&c[key]), q.bound64(mode, lin));
            assert!(
                (0..3).all(|k| (got[k] - want[k]).abs() <= b[k]),
                "the golden {key} of {s8:?} is {got:?}, qa's derivation from R-383 gives {want:?} (bound {b:?})"
            );
        }
    }
    assert!(
        colours.len() >= 216,
        "the golden set has {} colours",
        colours.len()
    );
    assert_eq!(
        seen_hatch,
        [true, true],
        "the golden set lacks a hatch colour"
    );
}

#[test]
fn qa_cvd_simulation_goldens_are_r383s_construction() {
    check_goldens(&qa());
}

negative_control!(
    qa_cvd_simulation_goldens_are_r383s_construction,
    "a Brettel whose colours take the other anchor's half-plane must not reproduce the goldens",
    expected = "the golden tritan of",
    {
        // The matrices stay qa's, so the matrices' checks pass and the colours' side rule is what fails.
        let mut q = qa();
        q.side_660 = -q.side_660;
        check_goldens(&q)
    }
);

/// dd_colouring §3.8's six-decimal transcription is qa's construction, to half a unit in the sixth decimal.
#[test]
fn qa_cvd_simulation_dd_colouring_transcription_is_r383s() {
    let q = qa();
    let half = 5e-7;
    close_m(
        "§3.8 protan",
        &[
            [0.112383, 0.887617, 0.0],
            [0.112383, 0.887617, 0.0],
            [0.004006, -0.004006, 1.0],
        ],
        &q.protan,
        half,
    );
    close_m(
        "§3.8 deutan",
        &[
            [0.292750, 0.707250, 0.0],
            [0.292750, 0.707250, 0.0],
            [-0.022337, 0.022337, 1.0],
        ],
        &q.deutan,
        half,
    );
    close_m(
        "§3.8 T1",
        &[
            [1.012773, 0.135485, -0.148257],
            [-0.012433, 0.868121, 0.144312],
            [0.075891, 0.805002, 0.119107],
        ],
        &q.t660,
        half,
    );
    close_m(
        "§3.8 T2",
        &[
            [0.936781, 0.189790, -0.126571],
            [0.061537, 0.815260, 0.123203],
            [-0.375624, 1.127665, 0.247958],
        ],
        &q.t485,
        half,
    );
    close_m(
        "§3.8 inverse",
        &[
            [8.094436, -13.050431, 11.672058],
            [-1.024851, 5.401931, -11.361471],
            [-0.036530, -0.412163, 69.351324],
        ],
        &q.inv,
        half,
    );
    let n_rgb: V3 = [0, 1, 2].map(|j| (0..3).map(|i| q.n_sep[i] * q.lms[i][j]).sum());
    // §3.8 gives the normal carried to linear RGB with 660 nm's side positive.
    assert!(
        q.side_660 > 0.0,
        "660 nm is not on the separating normal's positive side"
    );
    close_m(
        "§3.8 n_sep (RGB)",
        &[n_rgb; 3],
        &[[0.039015, -0.027881, -0.011134]; 3],
        half,
    );
}

negative_control!(
    qa_cvd_simulation_dd_colouring_transcription_is_r383s,
    "the Viénot transform built without the Judd–Vos correction (sRGB's XYZ) must not match §3.8",
    expected = "§3.8 protan",
    {
        let srgb_xyz = [
            [0.4124, 0.3576, 0.1805],
            [0.2126, 0.7152, 0.0722],
            [0.0193, 0.1192, 0.9505],
        ];
        let lms = mm(&QA_SP, &srgb_xyz);
        let n = cross(mv(&lms, [1.0, 1.0, 0.0]), mv(&lms, [0.0, 0.0, 1.0]));
        let protan = mm(&gauss_jordan(&lms), &mm(&project_along(n, 0), &lms));
        close_m(
            "§3.8 protan",
            &[
                [0.112383, 0.887617, 0.0],
                [0.112383, 0.887617, 0.0],
                [0.004006, -0.004006, 1.0],
            ],
            &protan,
            5e-7,
        )
    }
);

// ── REQ-COL-042: the crate's mirror against qa's derivation, off the goldens ────────────────────────────────────────

/// A 33-step lattice of linear values, and colours straddling Brettel's separating plane: for each lattice colour, the
/// colour moved onto the plane and a hair either side of it.
fn qa_colours(q: &Qa) -> Vec<V3> {
    let s = |k: usize| k as f64 / 32.0;
    let mut out: Vec<V3> = (0..33 * 33 * 33)
        .map(|k| [s(k / 1089), s(k / 33 % 33), s(k % 33)])
        .collect();
    // The separating normal on linear RGB, n_rgb · c = n_sep · LMS(c).
    let n_rgb: V3 = [0, 1, 2].map(|j| (0..3).map(|i| q.n_sep[i] * q.lms[i][j]).sum());
    let nn = dot(n_rgb, n_rgb);
    for k in (0..out.len()).step_by(97) {
        let c = out[k];
        let on = [0, 1, 2].map(|j| c[j] - dot(n_rgb, c) / nn * n_rgb[j]);
        for h in [-1e-9, 0.0, 1e-9] {
            out.push([0, 1, 2].map(|j| on[j] + h * n_rgb[j]));
        }
    }
    out
}

fn check_mirror(simulate: fn(CvdMode, V3) -> V3) {
    let q = qa();
    for c in qa_colours(&q) {
        for mode in CvdMode::ALL {
            let (want, got, b) = (q.sim(mode, c), simulate(mode, c), q.bound64(mode, c));
            assert!(
                (0..3).all(|k| (got[k] - want[k]).abs() <= b[k]),
                "the mirror's {} of {c:?} is {got:?}, qa's derivation gives {want:?} (bound {b:?})",
                mode.label()
            );
        }
    }
}

#[test]
fn qa_cvd_simulation_mirror_is_r383s_construction_off_the_lattice() {
    check_mirror(cvd::simulate);
}

negative_control!(
    qa_cvd_simulation_mirror_is_r383s_construction_off_the_lattice,
    "a mirror with protan and deutan crossed must fail",
    expected = "the mirror's deuteranopia of",
    check_mirror(|mode, c| match mode {
        CvdMode::Deuteranopia => cvd::simulate(CvdMode::Protanopia, c),
        _ => cvd::simulate(mode, c),
    })
);

// ── the method's properties (Viénot 1999, Brettel 1997), on the crate's mirror ─────────────────────────────────────

/// Under protan and deutan, the two kept cones are kept and the result lies on Viénot's plane; under tritan, L and M
/// are kept and the result lies on the half-plane of the colour's side; white is fixed by all three; each simulation
/// is its own image (a dichromat sees the simulated colour as it is).
fn check_properties(simulate: fn(CvdMode, V3) -> V3) {
    let q = qa();
    let tol = |x: V3| 1e-13 * (1.0 + x.iter().map(|v| v.abs()).sum::<f64>());
    for c in qa_colours(&q) {
        let l0 = mv(&q.lms, c);
        for (mode, missing) in [
            (CvdMode::Protanopia, 0usize),
            (CvdMode::Deuteranopia, 1),
            (CvdMode::Tritanopia, 2),
        ] {
            let s = simulate(mode, c);
            let l1 = mv(&q.lms, s);
            for k in (0..3).filter(|&k| k != missing) {
                assert!(
                    (l1[k] - l0[k]).abs() <= tol(l0),
                    "{} of {c:?} changes cone {k}: {} → {}",
                    mode.label(),
                    l0[k],
                    l1[k]
                );
            }
            let plane = match mode {
                CvdMode::Tritanopia => {
                    if dot(q.n_sep, l0) * q.side_660 >= 0.0 {
                        q.n_660
                    } else {
                        q.n_485
                    }
                }
                _ => q.n_vienot,
            };
            let off_plane = dot(plane, l1) / dot(plane, plane).sqrt();
            assert!(
                off_plane.abs() <= tol(l0),
                "{} of {c:?} is {off_plane:e} off its plane",
                mode.label()
            );
            let twice = simulate(mode, s);
            assert!(
                (0..3).all(|k| (twice[k] - s[k]).abs() <= tol(s)),
                "{} is not its own image at {c:?}: {s:?} → {twice:?}",
                mode.label()
            );
        }
    }
    for mode in [
        CvdMode::Protanopia,
        CvdMode::Deuteranopia,
        CvdMode::Tritanopia,
    ] {
        let w = simulate(mode, [1.0; 3]);
        assert!(
            w.iter().all(|x| (x - 1.0).abs() <= 1e-13),
            "{} moves white to {w:?}",
            mode.label()
        );
    }
    // Each Brettel anchor's linear colour is fixed by tritan.
    for a in [q.lms_660, q.lms_485] {
        let c = mv(&q.inv, a);
        let s = simulate(CvdMode::Tritanopia, c);
        assert!(
            (0..3).all(|k| (s[k] - c[k]).abs() <= 1e-13),
            "tritan moves its anchor {c:?} to {s:?}"
        );
    }
}

#[test]
fn qa_cvd_simulation_has_the_methods_properties() {
    check_properties(cvd::simulate);
}

negative_control!(
    qa_cvd_simulation_has_the_methods_properties,
    "a 'simulation' that only desaturates halfway to M_achrom's grey keeps no cone and must fail",
    expected = "changes cone",
    check_properties(|mode, c| match mode {
        CvdMode::Off => c,
        _ => {
            let y = dot(QA_ACHROM, c);
            c.map(|x| 0.5 * x + 0.5 * y)
        }
    })
);

// ── REQ-COL-042: achromatopsia is R = G = B exactly, and M_achrom's ─────────────────────────────────────────────────

fn check_achrom(simulate: fn(CvdMode, V3) -> V3) {
    let q = qa();
    for c in qa_colours(&q) {
        let g = simulate(CvdMode::Achromatopsia, c);
        assert!(
            g[0].to_bits() == g[1].to_bits() && g[1].to_bits() == g[2].to_bits(),
            "achromatopsia of {c:?} is {g:?}, not R = G = B exactly"
        );
        let want = dot(QA_ACHROM, c);
        assert!(
            (g[0] - want).abs() <= 4.0 * EPS * want.abs().max(EPS),
            "achromatopsia of {c:?} is {}, not M_achrom's {want}",
            g[0]
        );
    }
}

#[test]
fn qa_cvd_simulation_achrom_is_grey_exactly() {
    check_achrom(cvd::simulate);
}

negative_control!(
    qa_cvd_simulation_achrom_is_grey_exactly,
    "an achromatopsia computing each channel by its own summation order must fail R = G = B exactly",
    expected = "not R = G = B exactly",
    check_achrom(|_, c| {
        let [r, g, b] = c;
        [
            0.299 * r + 0.587 * g + 0.114 * b,
            0.114 * b + 0.587 * g + 0.299 * r,
            0.587 * g + 0.114 * b + 0.299 * r,
        ]
    })
);

// ── the pass on the GPU: REQ-COL-045's five modes, and the stage on linear colour ──────────────────────────────────

fn gpu() -> GpuHarness {
    GpuHarness::new().expect("a GPU device")
}

fn read_back(
    gpu: &GpuHarness,
    texture: &wgpu::Texture,
    bytes_per_texel: u32,
    width: u32,
) -> Vec<u8> {
    let (device, queue) = (gpu.device(), gpu.queue());
    let row = (bytes_per_texel * width).next_multiple_of(256);
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("qa readback"),
        size: u64::from(row),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &staging,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: None,
            },
        },
        wgpu::Extent3d {
            width,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);
    let slice = staging.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("maps"));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("polls");
    slice.get_mapped_range().expect("maps").to_vec()
}

/// The pass on an `Rgba32Float` layer of `colours` into an `Rgba32Float` screen, under the mode whose code is
/// `mode.code()`.
fn pass_float(gpu: &GpuHarness, colours: &[V3], mode: CvdMode) -> Vec<V3> {
    use wgpu::util::DeviceExt;
    let (device, queue) = (gpu.device(), gpu.queue());
    let width = colours.len() as u32;
    let format = wgpu::TextureFormat::Rgba32Float;
    let desc = |usage| wgpu::TextureDescriptor {
        label: Some("qa layer"),
        size: wgpu::Extent3d {
            width,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    };
    let texels: Vec<u8> = colours
        .iter()
        .flat_map(|c| [c[0] as f32, c[1] as f32, c[2] as f32, 1.0])
        .flat_map(f32::to_le_bytes)
        .collect();
    let source = device.create_texture_with_data(
        queue,
        &desc(wgpu::TextureUsages::TEXTURE_BINDING),
        wgpu::util::TextureDataOrder::LayerMajor,
        &texels,
    );
    let screen = device.create_texture(&desc(
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    ));
    let pass = CvdPass::new(device, format).unwrap_or_else(|e| panic!("{e}"));
    let mut encoder = device.create_command_encoder(&Default::default());
    pass.draw(
        device,
        &mut encoder,
        &source.create_view(&Default::default()),
        &screen.create_view(&Default::default()),
        mode,
    );
    queue.submit([encoder.finish()]);
    let bytes = read_back(gpu, &screen, 16, width);
    let f = |k: usize| f64::from(f32::from_le_bytes(bytes[k..k + 4].try_into().unwrap()));
    (0..colours.len())
        .map(|k| [f(16 * k), f(16 * k + 4), f(16 * k + 8)])
        .collect()
}

/// The GPU test colours: a 9-step lattice in [0, 1] (the pass's input is the gamut-clamped colour), rounded to f32 as
/// the float layer holds them.
fn gpu_colours() -> Vec<V3> {
    let s = |k: usize| f64::from((k as f64 / 8.0) as f32);
    (0..729)
        .map(|k| [s(k / 81), s(k / 9 % 9), s(k % 9)])
        .collect()
}

/// Each mode by its code transforms the colours as qa's derivation does, clamped to [0, 1] as the pass writes, within
/// the f32 bound `8 u (|T| |c|)ᵢ` (the matrix's and the colour's roundings to f32, three products and two sums); a
/// colour within that bound of Brettel's separating plane may take either half-plane. Off is the identity bit for bit;
/// achromatopsia is `R = G = B` exactly.
fn check_pass(gpu: &GpuHarness, expect: fn(&Qa, CvdMode, V3) -> V3) {
    let q = qa();
    let colours = gpu_colours();
    let n_rgb: V3 = [0, 1, 2].map(|j| (0..3).map(|i| q.n_sep[i] * q.lms[i][j]).sum());
    for mode in CvdMode::ALL {
        let got = pass_float(gpu, &colours, mode);
        for (c, g) in colours.iter().zip(&got) {
            let c = *c;
            match mode {
                CvdMode::Off => assert!(
                    (0..3).all(|k| g[k].to_bits() == c[k].to_bits()),
                    "the pass's off changes {c:?} to {g:?}"
                ),
                CvdMode::Achromatopsia => assert!(
                    g[0].to_bits() == g[1].to_bits() && g[1].to_bits() == g[2].to_bits(),
                    "the pass's achromatopsia of {c:?} is {g:?}, not R = G = B exactly"
                ),
                _ => {}
            }
            let row_bound = |t: &M3| mv(&abs_m(t), c).map(|x| 8.0 * U32 * x + 1e-15);
            let candidates: Vec<(V3, V3)> = match mode {
                CvdMode::Tritanopia
                    if dot(n_rgb, c).abs() <= 8.0 * U32 * mv(&abs_m(&[n_rgb; 3]), c)[0] =>
                {
                    vec![
                        (mv(&q.t660, c), row_bound(&q.t660)),
                        (mv(&q.t485, c), row_bound(&q.t485)),
                    ]
                }
                CvdMode::Achromatopsia => vec![(expect(&q, mode, c), row_bound(&[QA_ACHROM; 3]))],
                CvdMode::Off => vec![(expect(&q, mode, c), [0.0; 3])],
                _ => {
                    let t = match mode {
                        CvdMode::Protanopia => q.protan,
                        CvdMode::Deuteranopia => q.deutan,
                        _ => *q.tritan_matrix(c),
                    };
                    vec![(expect(&q, mode, c), row_bound(&t))]
                }
            };
            assert!(
                candidates.iter().any(|(want, b)| {
                    let want = want.map(|x| x.clamp(0.0, 1.0));
                    (0..3).all(|k| (g[k] - want[k]).abs() <= b[k])
                }),
                "the pass's {} (code {}) of {c:?} is {g:?}, qa's derivation gives {:?}",
                mode.label(),
                mode.code(),
                candidates[0].0
            );
        }
    }
}

#[test]
fn qa_cvd_modes_the_pass_runs_each_of_the_five_by_its_code() {
    check_pass(&gpu(), |q, mode, c| q.sim(mode, c));
}

negative_control!(
    qa_cvd_modes_the_pass_runs_each_of_the_five_by_its_code,
    "expecting deuteranopia where tritanopia's code runs must fail",
    expected = "the pass's tritanopia (code 3) of",
    check_pass(&gpu(), |q, mode, c| match mode {
        CvdMode::Tritanopia => q.sim(CvdMode::Deuteranopia, c),
        _ => q.sim(mode, c),
    })
);

/// REQ-COL-045: the modes offered are exactly render_gui_spec § "Display — the last stages"'s five, off, deuteranopia,
/// protanopia, tritanopia, achromatopsia (R-123), and no two simulate the same.
fn check_five(modes: &[CvdMode]) {
    let spec = std::fs::read_to_string(root().join("docs/gui/principia_render_gui_spec.md"))
        .expect("the spec");
    let line = spec
        .lines()
        .find(|l| l.contains("**Colour-vision simulation:**"))
        .expect("the spec's Display line");
    let listed: Vec<&str> = line
        .split(":**")
        .nth(1)
        .unwrap()
        .split('(')
        .next()
        .unwrap()
        .split(',')
        .map(str::trim)
        .collect();
    let labels: Vec<&str> = modes.iter().map(|m| m.label()).collect();
    assert_eq!(labels, listed, "the modes are not the spec's Display list");
    let probe = [0.8, 0.2, 0.1];
    let q = qa();
    for (i, a) in modes.iter().enumerate() {
        for b in &modes[i + 1..] {
            assert_ne!(
                q.sim(*a, probe),
                q.sim(*b, probe),
                "{} and {} simulate alike",
                a.label(),
                b.label()
            );
        }
    }
}

#[test]
fn qa_cvd_modes_are_the_specs_five() {
    check_five(&CvdMode::ALL);
}

negative_control!(
    qa_cvd_modes_are_the_specs_five,
    "the modes without achromatopsia (before R-123) must fail",
    expected = "the modes are not the spec's Display list",
    check_five(&CvdMode::ALL[..4])
);

/// The palette colours qa probes the stage with: the outcome palette, Okabe–Ito and the hatch's two (8-bit sRGB).
const PROBES: [[u8; 3]; 19] = [
    [0xDE, 0x2D, 0x2D],
    [0x2E, 0xBC, 0x4E],
    [0x34, 0x62, 0xE0],
    [0x14, 0x14, 0x18],
    [0xEC, 0xEC, 0xF0],
    [0xF0, 0xDE, 0x32],
    [0xE0, 0x34, 0xC6],
    [0x30, 0xC8, 0xDC],
    [0xF2, 0x96, 0x20],
    [0x00, 0x00, 0x00],
    [0xE6, 0x9F, 0x00],
    [0x56, 0xB4, 0xE9],
    [0x00, 0x9E, 0x73],
    [0xF0, 0xE4, 0x42],
    [0x00, 0x72, 0xB2],
    [0xD5, 0x5E, 0x00],
    [0xCC, 0x79, 0xA7],
    [0x9B, 0x00, 0xFF],
    [0x48, 0xFF, 0xFF],
];

/// The 8-bit screen value of an encoded channel, by qa's encode.
fn screen8(x: f64) -> i32 {
    (qa_encode(x) * 255.0).round() as i32
}

/// The probes whose screen, under `mode`, differs by more than one 8-bit step from qa's (decode, simulate, clamp,
/// encode, round), with the pass placed after linearisation (fed the decoded colour, its output encoded) or before it
/// (fed the encoded colour as it is stored, its output then taken as encoded). The pass runs on float layers, so no
/// backend sRGB conversion enters: the encode and decode around it are qa's, in f64. One step is the rounding
/// boundary: the pass's f32 error (`8 u (|T| |c|)ᵢ`, below 1e-6) can move a value across a rounding boundary, not
/// across two.
fn misses(gpu: &GpuHarness, mode: CvdMode, linear: bool) -> usize {
    let q = qa();
    let input: Vec<V3> = PROBES
        .iter()
        .map(|c| {
            if linear {
                c.map(qa_decode)
            } else {
                c.map(|x| f64::from(x) / 255.0)
            }
        })
        .collect();
    let got = pass_float(gpu, &input, mode);
    PROBES
        .iter()
        .zip(&got)
        .filter(|(c, g)| {
            let want = q.sim(mode, c.map(qa_decode)).map(screen8);
            let shown = g.map(|x| {
                if linear {
                    screen8(x)
                } else {
                    (x.clamp(0.0, 1.0) * 255.0).round() as i32
                }
            });
            (0..3).any(|k| (shown[k] - want[k]).abs() > 1)
        })
        .count()
}

/// Unit test 10's third clause: the stage on linear colour reproduces qa's screen within one 8-bit step under every
/// simulation, and the same pass applied before linearisation misses it on some probe under every simulation.
fn check_stage_placement(gpu: &GpuHarness, before: bool) {
    for mode in [
        CvdMode::Deuteranopia,
        CvdMode::Protanopia,
        CvdMode::Tritanopia,
        CvdMode::Achromatopsia,
    ] {
        let on_linear = misses(gpu, mode, true);
        assert_eq!(
            on_linear,
            0,
            "the stage on linear colour misses {on_linear} probes under {}",
            mode.label()
        );
        let pre = misses(gpu, mode, !before);
        eprintln!(
            "{} applied before linearisation: {pre} of {} probes differ",
            mode.label(),
            PROBES.len()
        );
        assert!(
            pre > 0,
            "{} applied before linearisation is not detected",
            mode.label()
        );
    }
}

#[test]
fn qa_cvd_simulation_prelinearisation_is_detected() {
    check_stage_placement(&gpu(), true);
}

negative_control!(
    qa_cvd_simulation_prelinearisation_is_detected,
    "comparing the stage on linear colour with itself detects nothing",
    expected = "applied before linearisation is not detected",
    check_stage_placement(&gpu(), false)
);

// ── REQ-COL-061: the proposal's numbers are reproducible ────────────────────────────────────────────────────────────

/// Ottosson's linear sRGB → OKLab (transcribed by qa for TASK-M7-02 from the pinned revision, `qa_TASK-M7-02.rs`).
fn qa_oklab(c: V3) -> V3 {
    let m1: M3 = [
        [0.4122214708, 0.5363325363, 0.0514459929],
        [0.2119034982, 0.6806995451, 0.1073969566],
        [0.0883024619, 0.2817188376, 0.6299787005],
    ];
    let m2: M3 = [
        [0.2104542553, 0.7936177850, -0.0040720468],
        [1.9779984951, -2.4285922050, 0.4505937099],
        [0.0259040371, 0.7827717662, -0.8086757660],
    ];
    mv(&m2, mv(&m1, c).map(f64::cbrt))
}

fn qa_dist(a: V3, b: V3) -> f64 {
    let (p, q) = (qa_oklab(a), qa_oklab(b));
    ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2)).sqrt()
}

/// The proposal's figures (the PR's table and `cvd_hatch_distinct`'s printout), to their four printed decimals: per
/// mode, violet's and cyan's least distance to a simulated entry, and the stripes' distance; `None` is off.
const PROPOSED: [(Option<CvdMode>, f64, f64, f64); 5] = [
    (None, 0.1415, 0.1117, 0.5054),
    (Some(CvdMode::Deuteranopia), 0.0454, 0.0189, 0.3965),
    (Some(CvdMode::Protanopia), 0.1056, 0.0156, 0.5354),
    (Some(CvdMode::Tritanopia), 0.0180, 0.0474, 0.3972),
    (Some(CvdMode::Achromatopsia), 0.0000, 0.0000, 0.3002),
];

/// qa's simulation, clamped as the pass writes, by qa's derivation; `None` is off.
type Sim = fn(&Qa, Option<CvdMode>, V3) -> V3;

fn qa_seen(q: &Qa, mode: Option<CvdMode>, c: V3) -> V3 {
    q.sim(mode.unwrap_or(CvdMode::Off), c)
        .map(|x| x.clamp(0.0, 1.0))
}

/// Reproduces every figure of the proposal with qa's simulation and OKLab over REQ-COL-055's palette list (the one
/// shared list, `support/palettes.rs`), and checks R-386's statements: the threshold is `#FF00FF`'s distance to
/// `#E034C6` (0.1006); under achromatopsia the stripes' OKLab lightnesses are 0.596 and 0.896, 0.300 apart; under every
/// simulation the stripes are farther apart than the threshold, and at least one stripe colour falls within it.
fn check_proposal(seen: Sim) {
    let q = qa();
    let hatch = ledger::gen::prelude::hatch().colours;
    assert_eq!(
        hatch,
        [[0x9B, 0x00, 0xFF], [0x48, 0xFF, 0xFF]],
        "the hatch is not R-136's violet and cyan"
    );
    let hatch = hatch.map(|c| c.map(qa_decode));
    let tau = qa_dist(
        [0xFF, 0x00, 0xFF].map(qa_decode),
        [0xE0, 0x34, 0xC6].map(qa_decode),
    );
    assert!(
        (tau - 0.1006).abs() < 5e-5,
        "the threshold is {tau:.5}, not the proposal's 0.1006"
    );
    let entries = palettes::entries();
    let round4 = |x: f64| (x * 1e4).round() / 1e4;
    for (mode, violet, cyan, stripes) in PROPOSED {
        let labs: Vec<V3> = entries
            .iter()
            .map(|(_, p)| qa_oklab(seen(&q, mode, *p)))
            .collect();
        let least = |h: V3| {
            let l = qa_oklab(seen(&q, mode, h));
            labs.iter()
                .map(|p| {
                    ((p[0] - l[0]).powi(2) + (p[1] - l[1]).powi(2) + (p[2] - l[2]).powi(2)).sqrt()
                })
                .fold(f64::INFINITY, f64::min)
        };
        let got = [
            least(hatch[0]),
            least(hatch[1]),
            qa_dist(seen(&q, mode, hatch[0]), seen(&q, mode, hatch[1])),
        ];
        let name = mode.map_or("off", CvdMode::label);
        eprintln!(
            "qa {name}: violet {:.4}, cyan {:.4}, stripes {:.4}",
            got[0], got[1], got[2]
        );
        assert_eq!(
            got.map(round4),
            [violet, cyan, stripes],
            "the proposal's {name} figures are not reproduced"
        );
        if mode.is_some() {
            assert!(
                got[2] > tau,
                "under {name} the stripes are only {:.4} apart",
                got[2]
            );
            assert!(
                got[0] <= tau || got[1] <= tau,
                "under {name} no stripe colour falls within the threshold, against the proposal's statement"
            );
        }
    }
    let l = hatch.map(|h| qa_oklab(qa_seen(&q, Some(CvdMode::Achromatopsia), h))[0]);
    assert!(
        (l[0] - 0.596).abs() < 5e-4
            && (l[1] - 0.896).abs() < 5e-4
            && (l[1] - l[0] - 0.300).abs() < 5e-4,
        "the achromatopsia stripes' lightnesses are {l:?}, not R-386's 0.596 and 0.896"
    );
}

#[test]
fn qa_cvd_hatch_distinct_the_proposals_figures_reproduce() {
    check_proposal(qa_seen);
}

negative_control!(
    qa_cvd_hatch_distinct_the_proposals_figures_reproduce,
    "a measure taken without clamping to what the pass writes must not reproduce the proposal",
    expected = "figures are not reproduced",
    check_proposal(|q, mode, c| q.sim(mode.unwrap_or(CvdMode::Off), c))
);
