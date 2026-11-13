//! qa's tests for TASK-M2-56, written from REQ-CHART-149 (R-895, ruling RQ-746 (a); R-513 Q1 to Q3; R-474), each with
//! its negative control (R-176).
//!
//! REQ-CHART-149: a new tilt of a saved curved chart into its own saved target, the saved tilt Lean each axis with one
//! target and both angles set, composes as R-513's `R·R′` literally, `R = [û₁ û₂ t̂]`, `t̂ = û₁ × û₂` normalised, its
//! sign toward `+e_X`; at the edge of Lean's range (`|τ₁|` or `|τ₂|` = 90° with the other set) `t̂` takes the cross
//! product's own sign; at `|τ₁| = |τ₂| = 90°` no composition is made; the saved target is offered (R-513 Q3).
//!
//! Every expected placement is computed here from the rulings' words, never read from the kernel: Lean each axis's
//! columns with one target `û₁ = (cos τ₁, 0, sin τ₁)`, `û₂ = (0, cos τ₂, sin τ₂)` in the scaled `(a, b, d̃_X)`
//! (R-474 R2; chart_decoder_contract § "Tilt (basis edit)"), `t̂` their cross product normalised, `R′` the new tilt's
//! own map, the hidden offset in units of its slider span (R-507 U1, R-513 Q2), each pixel's offset from the view's
//! centre (R-473 J7). The decode is checked to be the parent's own at the placed `(a, b)` with the control moved by the
//! placed offset, bit for bit.
//!
//! Tolerances: R-895's pixel is given to four decimals, so it is met to half a unit of the fourth, `5e-5`; the grid is
//! met "to 1 ulp", read as `|got − want| ≤ ε·max(|want|, Σ|terms|)`, the terms the products the matrices sum, fixed
//! before the run (R-649).

use kernel::chart::saved::Saved;
use kernel::chart::tilt::Hidden;
use kernel::chart::{Chart, Composed, ShapeMap, ShapeSphere, Tilt, TiltMap, TiltSpec, Tilted};
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

/// `(sin, cos)` of an angle in degrees, through the pinned libm the kernel's f64 path takes (R-991), so the reference
/// is the same on every machine.
fn sc(deg: f64) -> (f64, f64) {
    let r = deg.to_radians();
    (libm::sin(r), libm::cos(r))
}

fn bits(d: &Decoded<f64>) -> String {
    format!("{d:?}")
}

fn sphere() -> ShapeSphere<f64> {
    ShapeSphere {
        map: ShapeMap::Spherical,
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
fn window(c: [f64; 2], z: f64) -> Vec<[f64; 2]> {
    let half = 0.5 / z;
    let at = |k: u32, c: f64| c - half + 2.0 * half * (f64::from(k) + 0.5) / f64::from(N);
    (0..N)
        .flat_map(|j| (0..N).map(move |i| [at(i, c[0]), at(j, c[1])]))
        .collect()
}

// ── Matrices written from the rulings ───────────────────────────────────────────────────────────────────────────────

/// A 3 × 3 matrix on `(a, b, d̃_X)`, by rows.
type M3 = [[f64; 3]; 3];

fn mul(m: &M3, x: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| m[i][0] * x[0] + m[i][1] * x[1] + m[i][2] * x[2])
}

/// `|m|·|x|` per row: the scale of the products a row sums.
fn mag(m: &M3, x: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| (0..3).map(|k| (m[i][k] * x[k]).abs()).sum())
}

fn cross(p: [f64; 3], q: [f64; 3]) -> [f64; 3] {
    [
        p[1] * q[2] - p[2] * q[1],
        p[2] * q[0] - p[0] * q[2],
        p[0] * q[1] - p[1] * q[0],
    ]
}

fn dot(p: [f64; 3], q: [f64; 3]) -> f64 {
    p[0] * q[0] + p[1] * q[1] + p[2] * q[2]
}

/// How the saved Lean `R`'s third column is formed (the ruling, or a control).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Normal {
    /// R-895: `û₁ × û₂` normalised, its sign toward `+e_X`; where its `e_X` entry is 0 in exact arithmetic (the edge),
    /// the cross product's own sign.
    Ruled,
    /// The opposite sign.
    Flipped,
    /// `û₁ × û₂` unnormalised, its length the Gram factor.
    Unnormalised,
}

/// Lean each axis with one target: `[û₁ û₂ t̂]` as columns. `edge` says the angles are at the edge of Lean's range, where
/// the ruling keeps the cross product's own sign.
fn r_lean(t1: f64, t2: f64, how: Normal) -> M3 {
    let (s1, c1) = sc(t1);
    let (s2, c2) = sc(t2);
    let u1 = [c1, 0.0, s1];
    let u2 = [0.0, c2, s2];
    let n = cross(u1, u2);
    let len = dot(n, n).sqrt();
    let edge = t1.abs() == 90.0 || t2.abs() == 90.0;
    let toward = if !edge && n[2] < 0.0 { -1.0 } else { 1.0 };
    let t: [f64; 3] = match how {
        Normal::Ruled => n.map(|x| toward * x / len),
        Normal::Flipped => n.map(|x| -toward * x / len),
        Normal::Unnormalised => n.map(|x| toward * x),
    };
    std::array::from_fn(|i| [u1[i], u2[i], t[i]])
}

/// A single tilt `τ₁` of the horizontal axis into the target, the rotation in the `(a, d)` plane.
fn r_tau1(t: f64) -> M3 {
    let (s, c) = sc(t);
    [[c, 0.0, -s], [0.0, 1.0, 0.0], [s, 0.0, c]]
}

/// A single tilt `τ₂` of the vertical axis into the target, the rotation in the `(b, d)` plane.
fn r_tau2(t: f64) -> M3 {
    let (s, c) = sc(t);
    [[1.0, 0.0, 0.0], [0.0, c, -s], [0.0, s, c]]
}

/// Tip as sheet, one target, both angles set (R-474 K1): Rodrigues' rotation by `m = √(τ₁² + τ₂²)` about
/// `k = (τ₂ e_a − τ₁ e_b)/m`.
fn r_sheet(t1: f64, t2: f64) -> M3 {
    let m = (t1 * t1 + t2 * t2).sqrt();
    let k = [t2 / m, -t1 / m, 0.0];
    let (s, c) = sc(m);
    let kx = [[0.0, -k[2], k[1]], [k[2], 0.0, -k[0]], [-k[1], k[0], 0.0]];
    std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            let id = if i == j { 1.0 } else { 0.0 };
            id * c + kx[i][j] * s + k[i] * k[j] * (1.0 - c)
        })
    })
}

// ── The kernel's placement and the expected one ─────────────────────────────────────────────────────────────────────

/// The kernel's placement of pixel `(u, v)`: `(a, b)` and each hidden control's offset.
fn kernel_placed(
    view: &Tilted<ShapeSphere<f64>, f64, Composed<f64>>,
    u: f64,
    v: f64,
) -> ([f64; 2], [f64; 8]) {
    let (ab, shift) = view.at(u, v);
    let mut d = [0.0; 8];
    if let Some(s) = shift {
        d[s.x as usize] += s.dx;
        if s.two {
            d[s.y as usize] += s.dy;
        }
    }
    (ab, d)
}

/// The saved view `saved` through the serialisation, reopened at `S` under `new`; `None` where it is refused.
fn reopen(
    saved: &TiltSpec,
    new: &TiltSpec,
) -> Option<Tilted<ShapeSphere<f64>, f64, Composed<f64>>> {
    let chart = Saved::curved(&sphere(), *saved, S, [SPAN, SPAN]).expect("a curved parent saves");
    let Some(Saved::Curved(c)) = Saved::from_words(&chart.to_words()) else {
        panic!("the words do not read back as a curved chart");
    };
    c.reopen(sphere(), new, S, [SPAN, SPAN])
}

/// The new tilt: `R′·(ũ, ṽ, 0)` on its own target `X` (3 × 3), and, with a second target `X2`, `X2`'s offset.
#[derive(Clone, Copy)]
enum New {
    /// One target, the saved one, `R′` as a 3 × 3 matrix.
    Same(TiltSpec, M3),
    /// Two targets, `τ′₁` into the saved one, `τ′₂` into `X2` (the contract's two-target form).
    Two(TiltSpec),
}

impl New {
    fn spec(&self) -> TiltSpec {
        match *self {
            New::Same(t, _) | New::Two(t) => t,
        }
    }
}

/// The expected `(a, b, d)` and the scale of the terms each sums, at pixel `(u, v)` about `S`.
fn expected(r: &M3, new: New, u: f64, v: f64) -> ([f64; 2], [f64; 8], [f64; 2], [f64; 8]) {
    let p = [u - S[0], v - S[1], 0.0];
    let mut d = [0.0; 8];
    let mut scale_d = [0.0; 8];
    let prime = match new {
        New::Same(_, rn) => mul(&rn, p),
        New::Two(t) => {
            let (s1, c1) = sc(t.tau1);
            let (s2, c2) = sc(t.tau2);
            d[X2 as usize] = SPAN * s2 * p[1];
            scale_d[X2 as usize] = (SPAN * s2 * p[1]).abs();
            [c1 * p[0], c2 * p[1], s1 * p[0]]
        }
    };
    let out = mul(r, prime);
    let scale = mag(r, prime);
    d[X as usize] = SPAN * out[2];
    scale_d[X as usize] = SPAN * scale[2];
    (
        [S[0] + out[0], S[1] + out[1]],
        d,
        [S[0] + scale[0], S[1] + scale[1]],
        scale_d,
    )
}

/// Over a 33 × 33 window at zooms 1 and 4, the kernel's placement against `R·R′` with `R` the saved Lean
/// `[û₁ û₂ t̂]` formed as `how` says, to 1 ulp; the decode the parent's at the placement, bit for bit.
fn check_grid(saved: TiltSpec, new: New, how: Normal) {
    let r = r_lean(saved.tau1, saved.tau2, how);
    let view = reopen(&saved, &new.spec()).expect("the saved target is offered (R-513 Q3)");
    let mut worst: f64 = 0.0;
    for zoom in [1.0, 4.0] {
        for [u, v] in window(S, zoom) {
            let (ab, d) = kernel_placed(&view, u, v);
            let (want_ab, want_d, s_ab, s_d) = expected(&r, new, u, v);
            let got = ab.iter().chain(d.iter());
            let want = want_ab.iter().chain(want_d.iter());
            let scale = s_ab.iter().chain(s_d.iter());
            for ((g, w), s) in got.zip(want).zip(scale) {
                let unit = EPS * w.abs().max(*s);
                let err = (g - w).abs();
                assert!(
                    err <= unit || g == w,
                    "the placement at ({u}, {v}), zoom {zoom}, saved {saved:?}, is not R-895's R·R′: got {g:e}, \
                     want {w:e}"
                );
                if unit > 0.0 {
                    worst = worst.max(err / unit);
                }
            }
            let mut chart = sphere();
            for (k, dk) in d.iter().enumerate() {
                if *dk != 0.0 {
                    chart = chart
                        .sliced(k as u32, sphere().z_mom[k - 2] + dk)
                        .expect("a momentum control");
                }
            }
            assert!(
                bits(&view.decode::<Defaults, f64>(u, v))
                    == bits(&chart.decode::<Defaults, f64>(ab[0], ab[1])),
                "the decode at ({u}, {v}) is not the parent's at the placed (a, b) and offset"
            );
        }
    }
    println!("saved {saved:?}, new {:?}: worst {worst} ulp", new.spec());
}

// ── R-895's pixel ───────────────────────────────────────────────────────────────────────────────────────────────────

const SAVED: TiltSpec = TiltSpec {
    tau1: 30.0,
    tau2: 40.0,
    map: TiltMap::Lean,
    x: X,
    y: X,
};
const NEW: TiltSpec = TiltSpec {
    tau1: 20.0,
    tau2: 0.0,
    map: TiltMap::Sheet,
    x: X,
    y: X,
};

/// R-895's pixel: `(u, v) = (0.3, 0.7)` from the centre placed at `(Δa, Δb, d̃_X) = want` to the four decimals given,
/// the other controls untouched, under either map for the single new angle (R-474 R2: a single angle is the same
/// turn under both).
fn check_pixel(want: [f64; 3]) {
    for map in [TiltMap::Sheet, TiltMap::Lean] {
        let new = TiltSpec { map, ..NEW };
        let view = reopen(&SAVED, &new).expect("the saved target is offered (R-513 Q3)");
        let (ab, d) = kernel_placed(&view, S[0] + 0.3, S[1] + 0.7);
        let got = [ab[0] - S[0], ab[1] - S[1], d[X as usize] / SPAN];
        println!("R-895's pixel under {map:?}: {got:?}");
        assert!(
            (0..3).all(|i| (got[i] - want[i]).abs() <= 5e-5)
                && (0..8).all(|k| k == X as usize || d[k] == 0.0),
            "R-895's pixel is placed at {got:?}, offsets {d:?}, not {want:?}"
        );
    }
}

#[test]
fn qa_skewed_r895_pixel() {
    check_pixel([0.2026, 0.4759, 0.6628]);
}

negative_control!(
    qa_skewed_r895_pixel,
    "RQ-746's option (b), R-514's frame action, places the pixel at (0.1598, 0.4307, 0.7099), so it must fail",
    expected = "R-895's pixel is placed at",
    check_pixel([0.1598, 0.4307, 0.7099])
);

// ── Every pixel, R·R′ ───────────────────────────────────────────────────────────────────────────────────────────────

/// Saved Lean views across signs and quadrants, each with new tilts into the saved target: single angles under either
/// map, Tip as sheet with both set, Lean with both set, and two new targets (Q1 per target).
fn check_family(how: Normal) {
    for (t1, t2) in [(30.0, 40.0), (-25.0, 70.0), (55.0, -15.0), (-60.0, -35.0)] {
        let saved = spec(t1, t2, TiltMap::Lean, X, X);
        let news = [
            New::Same(NEW, r_tau1(20.0)),
            New::Same(spec(0.0, -30.0, TiltMap::Lean, X, X), r_tau2(-30.0)),
            New::Same(
                spec(-10.0, 30.0, TiltMap::Sheet, X, X),
                r_sheet(-10.0, 30.0),
            ),
            New::Same(
                spec(15.0, -20.0, TiltMap::Lean, X, X),
                r_lean(15.0, -20.0, Normal::Ruled),
            ),
            New::Two(spec(15.0, 20.0, TiltMap::Lean, X, X2)),
        ];
        for new in news {
            check_grid(saved, new, how);
        }
    }
}

#[test]
fn qa_skewed_composes_r_r_prime() {
    check_family(Normal::Ruled);
}

mod family_flipped {
    use super::*;
    negative_control!(
        qa_skewed_composes_r_r_prime,
        "t̂ with the opposite sign, away from +e_X, is not R-895's composition, so it must fail",
        expected = "is not R-895's R·R′",
        check_family(Normal::Flipped)
    );
}

mod family_unnormalised {
    use super::*;
    negative_control!(
        qa_skewed_composes_r_r_prime,
        "û₁ × û₂ left at its Gram length is not the unit normal R-895 names, so it must fail",
        expected = "is not R-895's R·R′",
        check_family(Normal::Unnormalised)
    );
}

// ── The edge of Lean's range ────────────────────────────────────────────────────────────────────────────────────────

/// `|τ₁|` or `|τ₂|` = 90° with the other set: the composition is made, `t̂` the cross product's own sign.
fn check_edge(how: Normal) {
    for (t1, t2) in [(90.0, 40.0), (-90.0, 40.0), (35.0, 90.0), (-50.0, -90.0)] {
        check_grid(
            spec(t1, t2, TiltMap::Lean, X, X),
            New::Same(NEW, r_tau1(20.0)),
            how,
        );
    }
}

#[test]
fn qa_skewed_edge_own_sign() {
    check_edge(Normal::Ruled);
}

negative_control!(
    qa_skewed_edge_own_sign,
    "at the edge, t̂ against the cross product's own sign is not R-895's, so it must fail",
    expected = "is not R-895's R·R′",
    check_edge(Normal::Flipped)
);

// ── RQ-471's corner: no composition ─────────────────────────────────────────────────────────────────────────────────

/// At `|τ₁| = |τ₂| = 90°` a new tilt into the saved target is not composed, alone or as one of two new targets.
/// `near` moves the saved angles a hair off the corner, where a frame exists (the control).
fn check_corner(near: bool) {
    let off = if near { 1e-6 } else { 0.0 };
    for (t1, t2) in [(90.0, 90.0f64), (90.0, -90.0), (-90.0, -90.0)] {
        let saved = spec(t1, t2 - off * t2.signum(), TiltMap::Lean, X, X);
        for new in [NEW, spec(15.0, 20.0, TiltMap::Lean, X, X2)] {
            assert!(
                reopen(&saved, &new).is_none(),
                "a new tilt into the target at RQ-471's corner, saved {saved:?}, new {new:?}, is composed"
            );
        }
    }
}

#[test]
fn qa_skewed_corner_refused() {
    check_corner(false);
}

negative_control!(
    qa_skewed_corner_refused,
    "a hair off the corner the plane has a normal and composes, so the refusal check must fail",
    expected = "is composed",
    check_corner(true)
);

// ── At τ′ = 0, the saved view ───────────────────────────────────────────────────────────────────────────────────────

/// Reopened under `new` (zero, or a control's hair), every pixel of the window decodes bit for bit as the saved view.
fn check_zero(new: TiltSpec) {
    for (t1, t2) in [(30.0, 40.0), (-25.0, 70.0), (90.0, 40.0)] {
        let saved = spec(t1, t2, TiltMap::Lean, X, X);
        let original =
            Tilted::new(sphere(), Tilt::new(&saved, S, [SPAN, SPAN])).expect("a hidden target");
        let view = reopen(&saved, &new).expect("reopens");
        for [u, v] in window(S, 1.0) {
            assert!(
                bits(&view.decode::<Defaults, f64>(u, v))
                    == bits(&original.decode::<Defaults, f64>(u, v)),
                "at τ′ = {new:?} the saved Lean view {saved:?} is not bitwise at ({u}, {v})"
            );
        }
    }
}

#[test]
fn qa_skewed_zero_is_saved_view() {
    check_zero(spec(0.0, 0.0, TiltMap::Sheet, X, X));
}

negative_control!(
    qa_skewed_zero_is_saved_view,
    "a hair of new tilt into the saved target moves the decode, so it must fail",
    expected = "is not bitwise",
    check_zero(spec(1e-7, 0.0, TiltMap::Sheet, X, X))
);

// ── The fixtures separate the controls ──────────────────────────────────────────────────────────────────────────────

/// Each control's `R` places some pixel of R-895's fixture, and of the edge's, far more than 1 ulp off the ruled one, so
/// no control can pass by accident of the fixture. `rules` lists the controls checked.
fn check_separates(rules: &[Normal]) {
    for (t1, t2) in [(30.0, 40.0), (90.0, 40.0), (-50.0, -90.0)] {
        let ruled = r_lean(t1, t2, Normal::Ruled);
        for &how in rules {
            let other = r_lean(t1, t2, how);
            let new = New::Same(NEW, r_tau1(20.0));
            let apart = window(S, 1.0).into_iter().any(|[u, v]| {
                let (a_ab, a_d, _, _) = expected(&ruled, new, u, v);
                let (b_ab, b_d, _, _) = expected(&other, new, u, v);
                let gap = (0..2)
                    .map(|i| (a_ab[i] - b_ab[i]).abs())
                    .chain((0..8).map(|k| (a_d[k] - b_d[k]).abs()))
                    .fold(0.0, f64::max);
                gap > 1e-3
            });
            assert!(
                apart,
                "the control {how:?} does not separate from R-895's at τ = ({t1}, {t2})"
            );
        }
    }
}

#[test]
fn qa_skewed_controls_separate() {
    check_separates(&[Normal::Flipped, Normal::Unnormalised]);
}

negative_control!(
    qa_skewed_controls_separate,
    "the ruled normal cannot separate from itself, so it must fail",
    expected = "does not separate",
    check_separates(&[Normal::Ruled])
);
