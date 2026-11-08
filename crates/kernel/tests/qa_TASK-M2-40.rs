//! qa's tests for TASK-M2-40, written from the two requirements it closes, each with its negative control (R-176).
//!
//! - REQ-CHART-073 (R-475 S2, S3, L5; R-477 C8): a latent view at `τ = (30°, 20°)` saved as the oblique chart
//!   `(z₀, q₁, q₂)`, the stored vectors exactly, reopened (through the serialisation) decodes bit-identically at every
//!   pixel of a 33 × 33 grid, under both tilt maps (control: re-orthonormalising `q₁`, `q₂` is not bitwise under Lean);
//!   a shape-sphere view tilted 25° into a momentum control saved and reopened at zero tilt likewise, on both shape
//!   maps (controls: the saved tilt dropped, and the slice centre dropped, each differ); a further tilt into a second
//!   momentum control decodes as the block form `(R ⊕ 1)·(1 ⊕ R′)` applied to `(u, v, 0, 0)` to 1 ulp (controls: the
//!   3 × 3 product `R·R′` feeding `d′` into `R`'s third input, and the order `R′·R`, each fail).
//! - REQ-CHART-077 (R-513 Q1 to Q3): a shape-sphere view tilted `τ = (25°, 0)` into a momentum control, saved, then
//!   tilted `τ′ = (0, 20°)` into the same control decodes as `R·R′·(u, v, 0)` to 1 ulp at every pixel of a 33 × 33
//!   grid at zoom 1 and zoom 4 alike; the same tilt into a second control is the block form; at `τ′ = 0` the saved
//!   view bit for bit (controls: the offsets added, `d + d′`, and the order `R′·R`, each fail; a hair of tilt is not
//!   bitwise). Beyond the verify's fixture: a saved Tip-as-sheet tilt with both angles set (Rodrigues' `R`, written here
//!   from R-474 K1, not read from the kernel), and Q1's per-target rule with two new targets, one the saved one.
//!
//! Every expected placement is computed here from the rulings' words: `R` and `R′` as 3 × 3 (or 4 × 4) rotations in
//! the scaled space `(a, b, d̃)`, the hidden offset in units of its slider span (R-507 U1, R-513 Q2), each pixel's
//! offset `(ũ, ṽ)` from the view's centre (R-473 J7), the view reopened at the saved centre. The decode is checked to be
//! the parent's own at the placed `(a, b)` with the control moved by the placed offset, bit for bit.
//!
//! "To 1 ulp" is read at the scale of the computation: `|got − want| ≤ ε·max(|want|, Σ|terms|)`, the terms being the
//! products the rotations sum, fixed before the run (R-649).

use kernel::chart::saved::Saved;
use kernel::chart::tilt::{latent_basis, Hidden, LatentFrame};
use kernel::chart::{
    Chart, Composed, Latent, ShapeMap, ShapeSphere, Tilt, TiltMap, TiltSpec, Tilted,
};
use kernel::decode::Decoded;
use kernel::generated::links::Defaults;
use validation::negative_control;

const EPS: f64 = f64::EPSILON;
/// The saved view's centre, in the parent's own parameters, off `(½, ½)`.
const S: [f64; 2] = [0.46, 0.57];
/// The slider span of every momentum control here.
const SPAN: f64 = 2.0;
/// The saved target, and a second momentum control.
const X: u32 = 2;
const X2: u32 = 3;
const N: u32 = 33;

fn spec(tau1: f64, tau2: f64, map: TiltMap, x: u32, y: u32) -> TiltSpec {
    TiltSpec {
        tau1,
        tau2,
        map,
        x,
        y,
    }
}

fn sc(deg: f64) -> (f64, f64) {
    deg.to_radians().sin_cos()
}

fn bits(d: &Decoded<f64>) -> String {
    format!("{d:?}")
}

fn sphere(map: ShapeMap) -> ShapeSphere<f64> {
    ShapeSphere {
        map,
        n0: [0.0, 0.0, 1.0],
        e1: [1.0, 0.0, 0.0],
        e2: [0.0, 1.0, 0.0],
        scale: 0.8,
        inertia: 1.0,
        phase: 0.3,
        z_mass: [-0.6, 0.25],
        z_mom: [0.1, 0.35, -0.2, -0.05],
    }
}

/// The pixel centres of an `n` × `n` window about `c` at zoom `z` (the window `c ± ½/z`).
fn window(c: [f64; 2], z: f64, n: u32) -> Vec<[f64; 2]> {
    let half = 0.5 / z;
    let at = |k: u32, c: f64| c - half + 2.0 * half * (f64::from(k) + 0.5) / f64::from(n);
    (0..n)
        .flat_map(|j| (0..n).map(move |i| [at(i, c[0]), at(j, c[1])]))
        .collect()
}

// ── Rotations written from the rulings ────────────────────────────────────────────────────────────────────────────

/// A 3 × 3 matrix on `(a, b, d)`, by rows.
type M3 = [[f64; 3]; 3];

fn mul(m: &M3, x: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| m[i][0] * x[0] + m[i][1] * x[1] + m[i][2] * x[2])
}

/// `|m|·|x|` per row: the scale of the products a row sums.
fn mag(m: &M3, x: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| (0..3).map(|k| (m[i][k] * x[k]).abs()).sum())
}

/// A single tilt `τ₁` of the horizontal axis into the target (R-473: `R·e_a = cos τ₁ e_a + sin τ₁ e_X`), the rotation in
/// the `(a, d)` plane.
fn r_tau1(t: f64) -> M3 {
    let (s, c) = sc(t);
    [[c, 0.0, -s], [0.0, 1.0, 0.0], [s, 0.0, c]]
}

/// A single tilt `τ₂` of the vertical axis into the target, the rotation in the `(b, d)` plane.
fn r_tau2(t: f64) -> M3 {
    let (s, c) = sc(t);
    [[1.0, 0.0, 0.0], [0.0, c, -s], [0.0, s, c]]
}

/// Tip as sheet, one target, both angles set (R-474 K1): the rotation by `m = √(τ₁² + τ₂²)` about the in-plane axis
/// `k = (τ₂ e_a − τ₁ e_b)/m`, Rodrigues' `R = I cos m + [k]× sin m + k kᵀ (1 − cos m)`.
fn r_sheet(t1: f64, t2: f64) -> M3 {
    let m = (t1 * t1 + t2 * t2).sqrt();
    let k = [t2 / m, -t1 / m, 0.0];
    let (s, c) = sc(m);
    let cross = [[0.0, -k[2], k[1]], [k[2], 0.0, -k[0]], [-k[1], k[0], 0.0]];
    std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            let id = if i == j { 1.0 } else { 0.0 };
            id * c + cross[i][j] * s + k[i] * k[j] * (1.0 - c)
        })
    })
}

/// A placement: the chart point `(a, b)` and each hidden control's offset, with the scale each was summed at.
#[derive(Clone, Copy, Debug)]
struct Placed {
    ab: [f64; 2],
    d: [f64; 8],
    scale_ab: [f64; 2],
    scale_d: [f64; 8],
}

/// The kernel's placement of pixel `(u, v)`.
fn kernel_placed<T: kernel::chart::tilt::Placement<f64>>(
    view: &Tilted<ShapeSphere<f64>, f64, T>,
    u: f64,
    v: f64,
) -> ([f64; 2], [f64; 8]) {
    let (ab, shift) = view.at(u, v);
    let mut d = [0.0; 8];
    if let Some(s) = shift {
        if (s.x as usize) < 8 {
            d[s.x as usize] += s.dx;
        }
        if s.two && (s.y as usize) < 8 {
            d[s.y as usize] += s.dy;
        }
    }
    (ab, d)
}

/// Whether the kernel's placement is the expected one to 1 ulp (the module's note); `Err` names the worst component.
fn within(got: ([f64; 2], [f64; 8]), want: &Placed) -> Result<f64, String> {
    let mut worst: f64 = 0.0;
    let mut check = |name: String, g: f64, w: f64, scale: f64| -> Result<(), String> {
        let unit = EPS * w.abs().max(scale);
        let err = (g - w).abs();
        if unit > 0.0 {
            worst = worst.max(err / unit);
        }
        if err <= unit || g == w {
            Ok(())
        } else {
            Err(format!("{name}: got {g:e}, want {w:e}, {} ulp", err / unit))
        }
    };
    for i in 0..2 {
        check(format!("ab[{i}]"), got.0[i], want.ab[i], want.scale_ab[i])?;
    }
    for k in 0..8 {
        check(format!("d[{k}]"), got.1[k], want.d[k], want.scale_d[k])?;
    }
    Ok(worst)
}

/// The decode at the placement: the parent at `(a, b)` with each control moved by its offset, bit for bit the view's.
fn check_decode<T: kernel::chart::tilt::Placement<f64>>(
    parent: &ShapeSphere<f64>,
    view: &Tilted<ShapeSphere<f64>, f64, T>,
    u: f64,
    v: f64,
) {
    let (ab, d) = kernel_placed(view, u, v);
    let mut chart = *parent;
    for (k, dk) in d.iter().enumerate() {
        if *dk != 0.0 {
            let z = parent.z_mom[k - 2];
            chart = chart.sliced(k as u32, z + dk).expect("a momentum control");
        }
    }
    assert!(
        bits(&view.decode::<Defaults, f64>(u, v))
            == bits(&chart.decode::<Defaults, f64>(ab[0], ab[1])),
        "the decode at ({u}, {v}) is not the parent's at the placed (a, b) and offsets"
    );
}

/// How the expected placement is formed (the requirement, or one of its controls).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Rule {
    /// The requirement: `R′` first, the new offset on the saved target into `R`'s third input, any other added.
    Ruled,
    /// RQ-403's option (a): `R` on `(u′, v′, 0)`, the new offset added to the saved one.
    OffsetsAdded,
    /// The order `R′·R`.
    Reversed,
    /// Into a second target, the 3 × 3 product `R·R′`: `d′` fed into `R`'s third input.
    Product3,
}

/// The expected placement of pixel `(u, v)` about `S`: the saved rotation `r` into `X`, the new rotation `r_new` into
/// `new_target` (in the scaled `(a, b, d̃)` of that target).
fn expected(r: &M3, r_new: &M3, new_target: u32, rule: Rule, u: f64, v: f64) -> Placed {
    let p = [u - S[0], v - S[1], 0.0];
    let mut d = [0.0; 8];
    let mut scale_d = [0.0; 8];
    let (out, scale) = match rule {
        Rule::Reversed => {
            let first = mul(r, p);
            (mul(r_new, first), mag(r_new, first))
        }
        _ => {
            let prime = mul(r_new, p);
            let same = new_target == X;
            let fed = match rule {
                Rule::Ruled => same,
                Rule::OffsetsAdded => false,
                Rule::Product3 => true,
                Rule::Reversed => unreachable!(),
            };
            let input = [prime[0], prime[1], if fed { prime[2] } else { 0.0 }];
            if !fed {
                let t = new_target as usize;
                d[t] += SPAN * prime[2];
                scale_d[t] += SPAN * mag(r_new, p)[2];
            }
            (mul(r, input), mag(r, input))
        }
    };
    d[X as usize] += SPAN * out[2];
    scale_d[X as usize] += SPAN * scale[2];
    Placed {
        ab: [S[0] + out[0], S[1] + out[1]],
        d,
        scale_ab: [S[0].abs() + scale[0], S[1].abs() + scale[1]],
        scale_d,
    }
}

/// The saved view `saved` of `parent` through the serialisation, reopened at `S` under `new`.
fn reopen(
    parent: ShapeSphere<f64>,
    saved: &TiltSpec,
    new: &TiltSpec,
) -> Tilted<ShapeSphere<f64>, f64, Composed<f64>> {
    let chart = Saved::curved(&parent, *saved, S, [SPAN, SPAN])
        .expect("the shape sphere saves as a curved chart");
    let read = Saved::from_words(&chart.to_words()).expect("the words read back");
    assert_eq!(
        read, chart,
        "the saved chart's words do not read back as the chart"
    );
    match read {
        Saved::Curved(c) => c
            .reopen(parent, new, S, [SPAN, SPAN])
            .expect("the new tilt's targets are hidden coordinates"),
        Saved::Oblique(_) => panic!("a curved view saved as an oblique chart"),
    }
}

/// The composition check over a 33 × 33 window at zooms 1 and 4: the kernel against `rule`'s placement.
fn check_composition(saved: TiltSpec, r: M3, new: TiltSpec, r_new: M3, rule: Rule) {
    let parent = sphere(ShapeMap::Spherical);
    let view = reopen(parent, &saved, &new);
    let mut worst: f64 = 0.0;
    for zoom in [1.0, 4.0] {
        for [u, v] in window(S, zoom, N) {
            let want = expected(&r, &r_new, new.x, rule, u, v);
            match within(kernel_placed(&view, u, v), &want) {
                Ok(w) => worst = worst.max(w),
                Err(e) => panic!(
                    "the placement at ({u}, {v}), zoom {zoom}, saved {saved:?}, new {new:?}, is not the composition: {e}"
                ),
            }
            check_decode(&parent, &view, u, v);
        }
    }
    println!("saved {saved:?}, new {new:?}: worst {worst} ulp");
}

// ── REQ-CHART-077: a new tilt into the saved target composes the rotations ──────────────────────────────────────────

const SAVED_25: TiltSpec = TiltSpec {
    tau1: 25.0,
    tau2: 0.0,
    map: TiltMap::Sheet,
    x: X,
    y: X,
};
const NEW_20: TiltSpec = TiltSpec {
    tau1: 0.0,
    tau2: 20.0,
    map: TiltMap::Sheet,
    x: X,
    y: X,
};

#[test]
fn qa_saved_same_target_composes() {
    check_composition(SAVED_25, r_tau1(25.0), NEW_20, r_tau2(20.0), Rule::Ruled);
}

mod same_offsets_added {
    use super::*;
    negative_control!(
        qa_saved_same_target_composes,
        "RQ-403's option (a), the offsets added d + d′, is not R-513's composition, so it must fail",
        expected = "is not the composition",
        check_composition(SAVED_25, r_tau1(25.0), NEW_20, r_tau2(20.0), Rule::OffsetsAdded)
    );
}

mod same_reversed {
    use super::*;
    negative_control!(
        qa_saved_same_target_composes,
        "the order R′·R differs for these non-parallel tilts, so it must fail",
        expected = "is not the composition",
        check_composition(SAVED_25, r_tau1(25.0), NEW_20, r_tau2(20.0), Rule::Reversed)
    );
}

/// A saved Tip-as-sheet tilt with both angles set, then a new tilt into the same target under either map, and a new
/// Tip-as-sheet tilt with both angles set: each `R·R′`, `R` from Rodrigues.
fn check_sheet(rule: Rule) {
    let saved = spec(25.0, 15.0, TiltMap::Sheet, X, X);
    let r = r_sheet(25.0, 15.0);
    for map in [TiltMap::Sheet, TiltMap::Lean] {
        check_composition(saved, r, spec(0.0, 20.0, map, X, X), r_tau2(20.0), rule);
        check_composition(saved, r, spec(-35.0, 0.0, map, X, X), r_tau1(-35.0), rule);
    }
    check_composition(
        saved,
        r,
        spec(-10.0, 30.0, TiltMap::Sheet, X, X),
        r_sheet(-10.0, 30.0),
        rule,
    );
}

#[test]
fn qa_saved_sheet_same_target_composes() {
    check_sheet(Rule::Ruled);
}

mod sheet_offsets_added {
    use super::*;
    negative_control!(
        qa_saved_sheet_same_target_composes,
        "the offsets added is not the composition under a saved Tip as sheet, so it must fail",
        expected = "is not the composition",
        check_sheet(Rule::OffsetsAdded)
    );
}

/// The composed map from `(ũ, ṽ)` to `(Δa, Δb, d̃)` is two rotations' product on `(ũ, ṽ, 0)`: its columns are
/// orthonormal, read off the kernel's placements by finite differences of a linear map (exact up to rounding).
fn check_orthonormal(perturb: f64) {
    let parent = sphere(ShapeMap::Spherical);
    let saved = spec(25.0, 15.0, TiltMap::Sheet, X, X);
    let new = spec(-10.0, 30.0, TiltMap::Sheet, X, X);
    let view = reopen(parent, &saved, &new);
    let h = 0.25;
    let col = |du: f64, dv: f64| {
        let (ab, d) = kernel_placed(&view, S[0] + du, S[1] + dv);
        let (ab0, d0) = kernel_placed(&view, S[0], S[1]);
        [
            (ab[0] - ab0[0]) / h,
            (ab[1] - ab0[1]) / h,
            (d[X as usize] - d0[X as usize]) / (SPAN * h) * (1.0 + perturb),
        ]
    };
    let (c1, c2) = (col(h, 0.0), col(0.0, h));
    let dot = |x: [f64; 3], y: [f64; 3]| x[0] * y[0] + x[1] * y[1] + x[2] * y[2];
    let tol = 64.0 * EPS;
    assert!(
        (dot(c1, c1) - 1.0).abs() <= tol
            && (dot(c2, c2) - 1.0).abs() <= tol
            && dot(c1, c2).abs() <= tol,
        "the composed columns {c1:?}, {c2:?} are not orthonormal"
    );
}

#[test]
fn qa_saved_composition_is_a_rotation() {
    check_orthonormal(0.0);
}

negative_control!(
    qa_saved_composition_is_a_rotation,
    "the hidden offset read at a slider span off by 1e-6 is no rotation's column, so it must fail",
    expected = "are not orthonormal",
    check_orthonormal(1e-6)
);

/// The fixture separates the rules (the verify's "non-parallel tilts, which do not commute"): at some pixel of the
/// window each control's placement is off the ruled one by far more than 1 ulp, so no control can pass by accident of
/// the fixture. `rules` lists the controls, each with the new tilt's target it is checked on.
fn check_separates(rules: &[(Rule, u32)]) {
    let r = r_tau1(25.0);
    let r_new = r_tau2(20.0);
    for &(rule, target) in rules {
        let apart = window(S, 1.0, N).into_iter().any(|[u, v]| {
            let a = expected(&r, &r_new, target, Rule::Ruled, u, v);
            let b = expected(&r, &r_new, target, rule, u, v);
            let gap = (0..2)
                .map(|i| (a.ab[i] - b.ab[i]).abs())
                .chain((0..8).map(|k| (a.d[k] - b.d[k]).abs()))
                .fold(0.0, f64::max);
            gap > 1e-3
        });
        assert!(
            apart,
            "the rule {rule:?} on target {target} does not separate from R-513's on this fixture"
        );
    }
}

#[test]
fn qa_saved_rules_separate() {
    check_separates(&[
        (Rule::OffsetsAdded, X),
        (Rule::Reversed, X),
        (Rule::Reversed, X2),
        (Rule::Product3, X2),
    ]);
}

negative_control!(
    qa_saved_rules_separate,
    "into a second target the offsets-added rule is the block form itself, so it cannot separate and must fail",
    expected = "does not separate",
    check_separates(&[(Rule::OffsetsAdded, X2)])
);

// ── REQ-CHART-073: into a second control, the block form ────────────────────────────────────────────────────────────

fn new_second() -> TiltSpec {
    spec(0.0, 20.0, TiltMap::Sheet, X2, X2)
}

#[test]
fn qa_saved_second_target_block_form() {
    check_composition(
        SAVED_25,
        r_tau1(25.0),
        new_second(),
        r_tau2(20.0),
        Rule::Ruled,
    );
    let saved = spec(25.0, 15.0, TiltMap::Sheet, X, X);
    check_composition(
        saved,
        r_sheet(25.0, 15.0),
        new_second(),
        r_tau2(20.0),
        Rule::Ruled,
    );
}

mod second_product3 {
    use super::*;
    negative_control!(
        qa_saved_second_target_block_form,
        "the 3 × 3 product R·R′, d′ fed into R's third input, is not the block form, so it must fail",
        expected = "is not the composition",
        check_composition(SAVED_25, r_tau1(25.0), new_second(), r_tau2(20.0), Rule::Product3)
    );
}

mod second_reversed {
    use super::*;
    negative_control!(
        qa_saved_second_target_block_form,
        "the order R′·R into a second target is not the block form, so it must fail",
        expected = "is not the composition",
        check_composition(
            SAVED_25,
            r_tau1(25.0),
            new_second(),
            r_tau2(20.0),
            Rule::Reversed
        )
    );
}

/// R-513 Q1 per target: a new two-target tilt (`τ′₁` into the saved target, `τ′₂` into a second control) feeds `R`'s
/// third input with the saved target's new offset and adds the other's to its own control. `fed_other` feeds the
/// second control's offset into `R` too (a control).
fn check_two_new(fed_other: bool) {
    let parent = sphere(ShapeMap::Spherical);
    let new = spec(15.0, 20.0, TiltMap::Lean, X, X2);
    let view = reopen(parent, &SAVED_25, &new);
    let r = r_tau1(25.0);
    let (s1, c1) = sc(15.0);
    let (s2, c2) = sc(20.0);
    for zoom in [1.0, 4.0] {
        for [u, v] in window(S, zoom, N) {
            let (du, dv) = (u - S[0], v - S[1]);
            // R′ on (ũ, ṽ, 0, 0) in (a, b, d_X, d_X2), the contract's two-target form.
            let prime = [c1 * du, c2 * dv, s1 * du, s2 * dv];
            let fed = if fed_other {
                prime[2] + prime[3]
            } else {
                prime[2]
            };
            let input = [prime[0], prime[1], fed];
            let out = mul(&r, input);
            let sc_out = mag(&r, input);
            let mut d = [0.0; 8];
            let mut scale_d = [0.0; 8];
            d[X as usize] = SPAN * out[2];
            scale_d[X as usize] = SPAN * sc_out[2];
            if !fed_other {
                d[X2 as usize] = SPAN * prime[3];
                scale_d[X2 as usize] = SPAN * prime[3].abs();
            }
            let want = Placed {
                ab: [S[0] + out[0], S[1] + out[1]],
                d,
                scale_ab: [S[0] + sc_out[0], S[1] + sc_out[1]],
                scale_d,
            };
            if let Err(e) = within(kernel_placed(&view, u, v), &want) {
                panic!(
                    "the two-target placement at ({u}, {v}), zoom {zoom}, is not R-513 Q1's: {e}"
                );
            }
            check_decode(&parent, &view, u, v);
        }
    }
}

#[test]
fn qa_saved_two_new_targets_per_target() {
    check_two_new(false);
}

negative_control!(
    qa_saved_two_new_targets_per_target,
    "the other target's new offset fed into R as well is not Q1's rule, so it must fail",
    expected = "is not R-513 Q1's",
    check_two_new(true)
);

// ── Reopened at zero tilt, bitwise (S3, C8, R-513 Q3) ─────────────────────────────────────────────────────────────

/// How the reopened chart is formed: as saved, or with one of the saved parts lost (each a control).
#[derive(Clone, Copy, PartialEq, Eq)]
struct Reopen {
    /// A hair of new tilt: not `τ′ = 0`, so the bitwise claim can fail.
    hair: bool,
    /// The saved tilt dropped.
    tilt_lost: bool,
    /// The slice centre's hidden values dropped (the parent's controls reset to 0).
    centre_lost: bool,
}

const AS_SAVED: Reopen = Reopen {
    hair: false,
    tilt_lost: false,
    centre_lost: false,
};

fn check_curved_bitwise(how: Reopen) {
    for shape in [ShapeMap::Spherical, ShapeMap::Exponential] {
        let parent = sphere(shape);
        for saved in [
            spec(25.0, 0.0, TiltMap::Sheet, X, X),
            spec(25.0, 15.0, TiltMap::Sheet, X, X),
            spec(25.0, 15.0, TiltMap::Lean, X, X),
            spec(25.0, -40.0, TiltMap::Lean, X, X2),
            spec(0.0, 0.0, TiltMap::Sheet, X, X),
        ] {
            let original = Tilted::new(parent, Tilt::new(&saved, S, [SPAN, SPAN]))
                .expect("a momentum control is hidden");
            let stored = if how.tilt_lost {
                spec(0.0, 0.0, TiltMap::Sheet, X, X)
            } else {
                saved
            };
            let mut reopened_parent = parent;
            if how.centre_lost {
                reopened_parent.z_mom = [0.0; 4];
            }
            let new = if how.hair {
                spec(1e-9, 0.0, TiltMap::Sheet, X, X)
            } else {
                spec(0.0, 0.0, TiltMap::Sheet, X, X)
            };
            let chart = Saved::curved(&parent, stored, S, [SPAN, SPAN]).expect("saves");
            let read = Saved::from_words(&chart.to_words()).expect("reads back");
            let Saved::Curved(c) = read else {
                panic!("a curved view read back as oblique")
            };
            let view = c
                .reopen(reopened_parent, &new, S, [SPAN, SPAN])
                .expect("reopens");
            for [u, v] in window(S, 1.0, N) {
                let want = original.decode::<Defaults, f64>(u, v);
                let got = view.decode::<Defaults, f64>(u, v);
                assert!(
                    bits(&got) == bits(&want),
                    "the reopened saved chart at ({u}, {v}), saved {saved:?}, {shape:?}, is not the saved view bitwise"
                );
            }
            if saved.untilted() && how == AS_SAVED {
                for [u, v] in window(S, 1.0, N) {
                    assert!(
                        bits(&view.decode::<Defaults, f64>(u, v))
                            == bits(&parent.decode::<Defaults, f64>(u, v)),
                        "an untilted saved view reopened is not the parent's own at ({u}, {v})"
                    );
                }
            }
        }
    }
}

#[test]
fn qa_saved_curved_reopens_bitwise() {
    check_curved_bitwise(AS_SAVED);
}

mod curved_hair {
    use super::*;
    negative_control!(
        qa_saved_curved_reopens_bitwise,
        "a new tilt of 1e-9 degrees is not τ′ = 0, so the bitwise check must fail",
        expected = "is not the saved view bitwise",
        check_curved_bitwise(Reopen {
            hair: true,
            ..AS_SAVED
        })
    );
}

mod curved_tilt_lost {
    use super::*;
    negative_control!(
        qa_saved_curved_reopens_bitwise,
        "a saved chart that drops its tilt is not the saved view, so it must fail",
        expected = "is not the saved view bitwise",
        check_curved_bitwise(Reopen {
            tilt_lost: true,
            ..AS_SAVED
        })
    );
}

mod curved_centre_lost {
    use super::*;
    negative_control!(
        qa_saved_curved_reopens_bitwise,
        "a saved chart that drops the slice centre's hidden values is not the saved view, so it must fail",
        expected = "is not the saved view bitwise",
        check_curved_bitwise(Reopen {
            centre_lost: true,
            ..AS_SAVED
        })
    );
}

// ── REQ-CHART-073: the latent view saved as the oblique chart ───────────────────────────────────────────────────────

/// Gram–Schmidt on `(q₁, q₂)`, each normalised: the re-orthonormalisation the requirement forbids.
fn orthonormalise(q1: [f64; 8], q2: [f64; 8]) -> ([f64; 8], [f64; 8]) {
    let dot = |x: &[f64; 8], y: &[f64; 8]| x.iter().zip(y).map(|(a, b)| a * b).sum::<f64>();
    let n1 = dot(&q1, &q1).sqrt();
    let e1 = q1.map(|x| x / n1);
    let p = dot(&q2, &e1);
    let r: [f64; 8] = std::array::from_fn(|i| q2[i] - p * e1[i]);
    let n2 = dot(&r, &r).sqrt();
    (e1, r.map(|x| x / n2))
}

fn check_latent(reorthonormalise: bool) {
    let frame = LatentFrame {
        a: 0,
        b: 1,
        scale: 0.9,
        gamma: 0.2,
    };
    let z0 = [0.15, -0.25, 0.3, -0.4, 0.2, 0.1, 0.5, -0.35];
    for map in [TiltMap::Lean, TiltMap::Sheet] {
        for (x, y) in [(4, 4), (4, 6)] {
            let (q1, q2) =
                latent_basis(&frame, &spec(30.0, 20.0, map, x, y)).expect("hidden targets");
            let view = Latent { z0, q1, q2 };
            let saved = Saved::latent(&view);
            let read = Saved::from_words(&saved.to_words()).expect("the words read back");
            let mut chart = read
                .oblique()
                .expect("a latent view saves as an oblique chart");
            assert!(
                chart.z0.map(f64::to_bits) == z0.map(f64::to_bits)
                    && chart.q1.map(f64::to_bits) == q1.map(f64::to_bits)
                    && chart.q2.map(f64::to_bits) == q2.map(f64::to_bits),
                "the oblique chart's vectors are not the stored ones exactly"
            );
            if reorthonormalise {
                let (o1, o2) = orthonormalise(chart.q1, chart.q2);
                chart.q1 = o1.map(|v| v * frame.scale);
                chart.q2 = o2.map(|v| v * frame.scale);
            }
            for [u, v] in window([0.5, 0.5], 1.0, N) {
                assert!(
                    bits(&chart.decode::<Defaults, f64>(u, v)) == bits(&view.decode::<Defaults, f64>(u, v)),
                    "the reopened oblique chart at ({u}, {v}) under {map:?}, targets ({x}, {y}), is not the view bitwise"
                );
            }
        }
    }
}

#[test]
fn qa_saved_latent_reopens_bitwise() {
    check_latent(false);
}

negative_control!(
    qa_saved_latent_reopens_bitwise,
    "a saved oblique chart that re-orthonormalises q₁, q₂ is not bitwise under Lean, so it must fail",
    expected = "is not the view bitwise",
    check_latent(true)
);

/// A latent view saves as an oblique chart, never as a curved chart on the latent parent.
#[test]
fn qa_saved_latent_is_not_curved() {
    let view = Latent {
        z0: [0.0; 8],
        q1: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        q2: [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    };
    assert!(Saved::curved(
        &view,
        spec(10.0, 0.0, TiltMap::Sheet, 4, 4),
        [0.5, 0.5],
        [SPAN, SPAN]
    )
    .is_none());
    assert!(Saved::latent(&view).oblique().is_some());
    let curved =
        Saved::curved(&sphere(ShapeMap::Spherical), SAVED_25, S, [SPAN, SPAN]).expect("saves");
    assert!(
        curved.oblique().is_none(),
        "a curved saved chart reopened as an oblique one"
    );
}
