//! Saved charts, the rest (`kernel::chart::saved`): compositions onto three or four hidden coordinates, a saved chart
//! saved again, and the composed chart's area weight and repeat counting.
//!
//! - `saved_chart_wide_as_today`: every composition of at most two hidden coordinates places, decodes and serialises
//!   to a fixed digest of its bits.
//! - `saved_chart_wide_targets`: a new tilt onto three or four hidden coordinates is `R·R′`, each offset on its own
//!   target, to one ulp of the terms that form it, and decodes at `(a, b)` with every offset on its control.
//! - `saved_chart_wide_refused`: five or six coordinates, and an offset into a tilt with no input there, are refused.
//! - `saved_chart_wide_chain`: a saved chart saved again is `R·R′·R″`; reopened at `τ″ = 0` it is the view it was
//!   saved from, bit for bit; its two records read back bit for bit and nothing else reads as a chain.
//! - `saved_chart_wide_weight`: the composed chart's area weight is 1 for rotations and the area of the window's steps
//!   with a skewed tilt; `ray-degenerate` where the innermost tilt is at the corner.
//! - `saved_chart_wide_repeats`: with one tilt on, its fold exactly; with several, only where every tilt keeps every
//!   offset; and the antipode's one system counts once per distinct offset on all four slots.
//!
//! "To 1 ulp" here: each placed value differs from the reference by at most `ε·Σ|terms|` of the terms that form it, a
//! criterion fixed before the run. Each test registers its negative control.

use kernel::chart::saved::{Chain, Composed, Level, Saved, WORDS};
use kernel::chart::shape_sphere::PixelCount;
use kernel::chart::tilt::{Folds, Hidden, Placement, Shift, Tilt};
use kernel::chart::{Chart, Float, ShapeMap, ShapeSphere, SystemImage, TiltMap, TiltSpec, Tilted};
use kernel::generated::links::Defaults;
use validation::negative_control;

const CENTRE: [f64; 2] = [0.37, 0.61];
const X: u32 = 2;

fn spec(tau1: f64, tau2: f64, map: TiltMap, x: u32, y: u32) -> TiltSpec {
    TiltSpec {
        tau1,
        tau2,
        map,
        x,
        y,
    }
}

fn sphere() -> ShapeSphere<f64> {
    ShapeSphere {
        map: ShapeMap::Spherical,
        n0: [1.0, 0.0, 0.0],
        e1: [0.0, 1.0, 0.0],
        e2: [0.0, 0.0, 1.0],
        scale: 1.0,
        inertia: 1.0,
        phase: 0.0,
        z_mass: [0.3, -0.4],
        z_mom: [0.2, -0.1, 0.15, 0.05],
    }
}

/// FNV-1a over 64-bit words.
struct Digest(u64);

impl Digest {
    fn new() -> Self {
        Digest(0xcbf2_9ce4_8422_2325)
    }
    fn word(&mut self, w: u64) {
        for byte in w.to_le_bytes() {
            self.0 = (self.0 ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
        }
    }
    fn float(&mut self, x: f64) {
        self.word(x.to_bits());
    }
    fn text(&mut self, s: &str) {
        for byte in s.bytes() {
            self.word(u64::from(byte));
        }
    }
}

/// The compositions of at most two hidden coordinates: (saved, saved centre, saved span, new, centre, span).
/// A saved tilt with its centre and spans, then a new one.
type Pair = (TiltSpec, [f64; 2], [f64; 2], TiltSpec, [f64; 2], [f64; 2]);

fn cases() -> Vec<Pair> {
    let span = [2.0, 2.0];
    let odd = [1.5, 2.5];
    let skewed = spec(30.0, 40.0, TiltMap::Lean, 4, 4);
    vec![
        (
            spec(25.0, 0.0, TiltMap::Sheet, X, X),
            CENTRE,
            span,
            spec(10.0, 0.0, TiltMap::Sheet, X, X),
            CENTRE,
            span,
        ),
        (
            spec(25.0, 0.0, TiltMap::Sheet, X, X),
            CENTRE,
            span,
            spec(10.0, 5.0, TiltMap::Lean, 3, 3),
            [0.42, 0.55],
            span,
        ),
        (
            spec(25.0, 0.0, TiltMap::Sheet, X, X),
            CENTRE,
            span,
            spec(10.0, 5.0, TiltMap::Lean, X, 3),
            [0.42, 0.55],
            odd,
        ),
        (
            spec(10.0, 5.0, TiltMap::Lean, 2, 3),
            CENTRE,
            odd,
            spec(10.0, 0.0, TiltMap::Sheet, 3, 3),
            CENTRE,
            span,
        ),
        (
            spec(10.0, 5.0, TiltMap::Lean, 2, 3),
            CENTRE,
            odd,
            spec(-12.0, 7.0, TiltMap::Lean, 3, 2),
            [0.5, 0.5],
            odd,
        ),
        (
            skewed,
            CENTRE,
            span,
            spec(10.0, 0.0, TiltMap::Sheet, 4, 4),
            [0.4, 0.6],
            span,
        ),
        (
            skewed,
            CENTRE,
            span,
            spec(10.0, 0.0, TiltMap::Sheet, 5, 5),
            CENTRE,
            odd,
        ),
        (
            skewed,
            CENTRE,
            span,
            spec(10.0, 5.0, TiltMap::Lean, 5, 4),
            CENTRE,
            span,
        ),
        (
            spec(20.0, 15.0, TiltMap::Sheet, 2, 2),
            CENTRE,
            span,
            spec(-12.0, 18.0, TiltMap::Sheet, 2, 2),
            [0.3, 0.7],
            span,
        ),
        (
            spec(-90.0, 90.0, TiltMap::Lean, 4, 4),
            CENTRE,
            span,
            spec(10.0, 0.0, TiltMap::Sheet, 5, 5),
            CENTRE,
            span,
        ),
        (
            spec(25.0, 0.0, TiltMap::Sheet, X, X),
            CENTRE,
            span,
            spec(0.0, 0.0, TiltMap::Sheet, X, X),
            CENTRE,
            span,
        ),
        (
            spec(0.0, 0.0, TiltMap::Sheet, X, X),
            CENTRE,
            span,
            spec(10.0, 5.0, TiltMap::Lean, 2, 3),
            [0.45, 0.5],
            odd,
        ),
    ]
}

/// The digest of every composition's placement (f64 and f32, the offsets of the first two targets), its decode on the
/// sphere, and the saved definitions' words.
fn as_today(flip: bool) -> u64 {
    let mut d = Digest::new();
    for (s, sc, ss, n, nc, ns) in cases() {
        let Some(c) = Composed::new(&s, sc, ss, &n, nc, ns) else {
            d.word(u64::MAX);
            continue;
        };
        let narrow = c.at_precision::<f32>();
        for j in 0..9u32 {
            for i in 0..9u32 {
                let (u, v) = (0.08 + 0.105 * f64::from(i), 0.07 + 0.11 * f64::from(j));
                let (ab, sh): ([f64; 2], Shift<f64>) = c.place(u, v);
                d.float(ab[0]);
                d.float(ab[1]);
                let shifts = [sh.x, sh.y];
                for k in shifts {
                    d.word(u64::from(k));
                }
                d.word(u64::from(sh.two));
                d.float(sh.dx);
                if sh.two {
                    d.float(sh.dy);
                }
                let (ab, sh): ([f32; 2], Shift<f32>) = narrow.place(u as f32, v as f32);
                d.float(f64::from(ab[0]));
                d.float(f64::from(ab[1]));
                d.float(f64::from(sh.dx));
                if sh.two {
                    d.float(f64::from(sh.dy));
                }
            }
        }
        if let Some(view) = Tilted::placed(sphere(), c) {
            for (u, v) in [(0.5, 0.5), (0.31, 0.77), (0.9, 0.12)] {
                d.text(&format!("{:?}", view.decode::<Defaults, f64>(u, v)));
            }
        }
    }
    for parent in [sphere()] {
        for (s, sc, ss, ..) in cases() {
            if let Some(def) = Saved::curved(&parent, s, sc, ss) {
                for w in def.to_words() {
                    d.float(w);
                }
            }
        }
    }
    if flip {
        d.word(1);
    }
    d.0
}

/// Digest of the placement bits, decode and words of the twelve two-coordinate compositions in `cases()`. Don't
/// re-capture it to make a failure pass.
const TODAY: u64 = 11_074_953_001_277_844_569;

#[test]
fn saved_chart_wide_as_today() {
    assert_eq!(as_today(false), TODAY, "a composition changed its bits");
}

negative_control!(
    saved_chart_wide_as_today,
    "a one-bit change in the digest's input must not match",
    expected = "a composition changed its bits",
    {
        assert_eq!(as_today(true), TODAY, "a composition changed its bits");
    }
);

// ── The reference: R·R′·R″ as matrices on (a, b, h₀ … h₇) ──────────────────────────────────────────────────────────

type M = [[f64; 10]; 10];

fn sc(deg: f64) -> (f64, f64) {
    Float::sin_cos(Float::to_radians(deg))
}

fn givens(m: &mut M, i: usize, j: usize, deg: f64) {
    let (s, c) = sc(deg);
    m[i][i] = c;
    m[j][i] = s;
    m[i][j] = -s;
    m[j][j] = c;
}

/// A stored tilt's matrix on `(a, b, h₀ … h₇)`, from its closed forms (two targets: each axis turned toward its own;
/// Tip as sheet with both angles: Rodrigues about `k = (τ₂ e_a − τ₁ e_b)/m`; Lean with one target and both angles:
/// `[û₁ û₂ t̂]`; one angle: its axis's turn).
fn matrix(t: &TiltSpec) -> M {
    let mut m = [[0.0; 10]; 10];
    for (k, row) in m.iter_mut().enumerate() {
        row[k] = 1.0;
    }
    let (x, y) = (2 + t.x as usize, 2 + t.y as usize);
    if t.x != t.y {
        givens(&mut m, 0, x, t.tau1);
        givens(&mut m, 1, y, t.tau2);
    } else if t.tau1 != 0.0 && t.tau2 != 0.0 && t.map == TiltMap::Lean {
        let (s1, c1) = sc(t.tau1);
        let (s2, c2) = sc(t.tau2);
        let cross = [-s1 * c2, -c1 * s2, c1 * c2];
        let length = Float::sqrt(cross.iter().map(|c| c * c).sum::<f64>());
        let toward = if cross[2] < 0.0 { -1.0 } else { 1.0 };
        let columns = [
            [c1, 0.0, s1],
            [0.0, c2, s2],
            cross.map(|c| toward * c / length),
        ];
        let at = [0, 1, x];
        for r in 0..3 {
            for q in 0..3 {
                m[at[r]][at[q]] = columns[q][r];
            }
        }
    } else if t.tau1 != 0.0 && t.tau2 != 0.0 {
        let deg = Float::sqrt(t.tau1 * t.tau1 + t.tau2 * t.tau2);
        let k = [t.tau2 / deg, -t.tau1 / deg, 0.0];
        let (s, c) = sc(deg);
        let cross = [[0.0, -k[2], k[1]], [k[2], 0.0, -k[0]], [-k[1], k[0], 0.0]];
        let at = [0, 1, x];
        for r in 0..3 {
            for q in 0..3 {
                let id = if r == q { c } else { 0.0 };
                m[at[r]][at[q]] = id + s * cross[r][q] + (1.0 - c) * k[r] * k[q];
            }
        }
    } else if t.tau2 == 0.0 {
        givens(&mut m, 0, x, t.tau1);
    } else {
        givens(&mut m, 1, x, t.tau2);
    }
    m
}

/// `m·x`, and for each component the sum of the magnitudes of every product that forms it, `size` carrying `x`'s own.
fn apply(m: &M, x: &[f64; 10], size: &[f64; 10]) -> ([f64; 10], [f64; 10]) {
    let mut y = [0.0; 10];
    let mut terms = [0.0; 10];
    for r in 0..10 {
        for q in 0..10 {
            y[r] += m[r][q] * x[q];
            terms[r] += m[r][q].abs() * (x[q].abs() + size[q]);
        }
    }
    (y, terms)
}

/// A pixel's placement: `(a, b)` and the offset on each latent control.
#[derive(Clone, Copy, Debug)]
struct Placed {
    ab: [f64; 2],
    d: [f64; 8],
}

/// The reference placement of the pixel `(u, v)` under `levels`, outermost first, each tilt's matrix applied to the
/// one inside it, the centres' differences added between; every offset times its control's `span`; with each value's
/// tolerance, `ε` times the sum of the magnitudes of every product and sum that forms it. `order` picks which tilt acts
/// first: 0 the innermost (the composition), 1 the outermost (the control's wrong order).
fn reference(
    levels: &[(TiltSpec, [f64; 2])],
    span: f64,
    u: f64,
    v: f64,
    order: usize,
) -> (Placed, Placed) {
    let mut seq: Vec<(TiltSpec, [f64; 2])> = levels.iter().rev().copied().collect();
    if order == 1 {
        seq.reverse();
    }
    let mut x = [0.0; 10];
    x[0] = u - seq[0].1[0];
    x[1] = v - seq[0].1[1];
    let mut size = x.map(f64::abs);
    let (mut y, mut terms) = apply(&matrix(&seq[0].0), &x, &size);
    for k in 1..seq.len() {
        for q in 0..2 {
            let pan = seq[k - 1].1[q] - seq[k].1[q];
            y[q] += pan;
            terms[q] += pan.abs() + y[q].abs();
        }
        size = terms;
        let (z, t) = apply(&matrix(&seq[k].0), &y, &size);
        y = z;
        terms = t;
    }
    let outer = seq[seq.len() - 1].1;
    let ab = [outer[0] + y[0], outer[1] + y[1]];
    let tol_ab = [
        f64::EPSILON * (outer[0].abs() + terms[0] + ab[0].abs()),
        f64::EPSILON * (outer[1].abs() + terms[1] + ab[1].abs()),
    ];
    let mut d = [0.0; 8];
    let mut tol_d = [0.0; 8];
    for k in 0..8 {
        d[k] = span * y[2 + k];
        tol_d[k] = f64::EPSILON * span * terms[2 + k];
    }
    (
        Placed { ab, d },
        Placed {
            ab: tol_ab,
            d: tol_d,
        },
    )
}

/// A kernel shift as one offset per latent control, every slot read as its count says.
fn offsets(shift: &Shift<f64>) -> [f64; 8] {
    let mut d = [0.0; 8];
    d[shift.x as usize] += shift.dx;
    if shift.two {
        d[shift.y as usize] += shift.dy;
    }
    if shift.more >= 1 {
        d[shift.z as usize] += shift.dz;
    }
    if shift.more >= 2 {
        d[shift.w as usize] += shift.dw;
    }
    d
}

/// The hidden coordinates a composition names.
fn named(c: &Composed<f64>) -> usize {
    let (_, two) = c.targets();
    let (_, more) = c.more_targets();
    1 + usize::from(two) + more as usize
}

fn within(got: &Placed, want: &Placed, tol: &Placed) -> bool {
    let close = |g: f64, w: f64, t: f64| (g - w).abs() <= t;
    (0..2).all(|k| close(got.ab[k], want.ab[k], tol.ab[k]))
        && (0..8).all(|k| close(got.d[k], want.d[k], tol.d[k]))
}

const SPAN: [f64; 2] = [2.0, 2.0];

fn window() -> Vec<[f64; 2]> {
    let mut points = Vec::new();
    for j in 0..9u32 {
        for i in 0..9u32 {
            points.push([0.06 + 0.11 * f64::from(i), 0.05 + 0.115 * f64::from(j)]);
        }
    }
    points
}

fn level(tilt: TiltSpec, centre: [f64; 2]) -> Level {
    Level {
        tilt,
        centre,
        span: SPAN,
    }
}

/// How the check forms the pixel's placement: the kernel's, or a control's wrong one.
#[derive(Clone, Copy, PartialEq)]
enum Got {
    Kernel,
    /// The tilts in the opposite order, `R′·R`.
    Reversed,
    /// The kernel's, the third and fourth offsets dropped.
    Dropped,
}

/// Every pixel of the 9 × 9 window under `levels` (outermost first) places as the chain of matrices to one ulp, names
/// `want` hidden coordinates, and decodes at `(a, b)` with each offset on its control.
fn check_chain(levels: &[Level], want: usize, got: Got) {
    let composed = Composed::of(levels).expect("a composition");
    check_composed(composed, levels, want, got);
}

/// [`check_chain`] on the composition `composed`, built however the caller built it from `levels`.
fn check_composed(composed: Composed<f64>, levels: &[Level], want: usize, got: Got) {
    assert_eq!(named(&composed), want, "the coordinates named");
    let spec: Vec<_> = levels.iter().map(|l| (l.tilt, l.centre)).collect();
    let view = Tilted::placed(sphere(), composed).expect("every target a hidden control");
    for [u, v] in window() {
        let (ab, shift) = composed.place::<f64>(u, v);
        let mut shift = shift;
        if got == Got::Dropped {
            shift.more = 0;
        }
        let have = Placed {
            ab,
            d: offsets(&shift),
        };
        let order = usize::from(got == Got::Reversed);
        let (expect, tol) = reference(&spec, SPAN[0], u, v, order);
        assert!(
            within(&have, &expect, &tol),
            "the composition at ({u}, {v}) is not R·R′: {have:?} against {expect:?}"
        );
        let mut chart = sphere();
        for (k, d) in have.d.iter().enumerate() {
            if *d != 0.0 {
                let held = chart.map(0.5, 0.5).z.z[k];
                chart = chart.sliced(k as u32, held + d).expect("a hidden control");
            }
        }
        if got == Got::Kernel {
            let [a, b] = have.ab;
            assert_eq!(
                format!("{:?}", view.decode::<Defaults, f64>(u, v)),
                format!("{:?}", chart.decode::<Defaults, f64>(a, b)),
                "the composition at ({u}, {v}) does not decode at (a, b) with its offsets on the controls"
            );
        }
    }
}

fn wide_cases() -> Vec<(Vec<Level>, usize)> {
    let at = [0.42, 0.55];
    vec![
        // Saved onto 2, new onto 4 and 5: three coordinates.
        (
            vec![
                level(spec(25.0, 0.0, TiltMap::Sheet, 2, 2), CENTRE),
                level(spec(10.0, 5.0, TiltMap::Lean, 4, 5), at),
            ],
            3,
        ),
        // Saved onto 2 and 3, new onto 4: three.
        (
            vec![
                level(spec(10.0, 5.0, TiltMap::Lean, 2, 3), CENTRE),
                level(spec(20.0, 0.0, TiltMap::Sheet, 4, 4), at),
            ],
            3,
        ),
        // Saved onto 2 and 3, new onto 4 and 5: four.
        (
            vec![
                level(spec(10.0, 5.0, TiltMap::Lean, 2, 3), CENTRE),
                level(spec(8.0, 6.0, TiltMap::Lean, 4, 5), at),
            ],
            4,
        ),
        // One shared: saved 2, 3; new 2, 4: three.
        (
            vec![
                level(spec(10.0, 5.0, TiltMap::Lean, 2, 3), CENTRE),
                level(spec(8.0, 6.0, TiltMap::Lean, 2, 4), at),
            ],
            3,
        ),
        // A skewed saved tilt onto 4, new onto 5 and 6: three.
        (
            vec![
                level(spec(30.0, 40.0, TiltMap::Lean, 4, 4), CENTRE),
                level(spec(10.0, 5.0, TiltMap::Lean, 5, 6), at),
            ],
            3,
        ),
        // Tip as sheet saved onto 4, new onto 5 and 6 (masses): three.
        (
            vec![
                level(spec(20.0, 15.0, TiltMap::Sheet, 4, 4), CENTRE),
                level(spec(-12.0, 9.0, TiltMap::Lean, 5, 6), at),
            ],
            3,
        ),
    ]
}

#[test]
fn saved_chart_wide_targets() {
    for (levels, want) in wide_cases() {
        check_chain(&levels, want, Got::Kernel);
    }
}

negative_control!(
    saved_chart_wide_targets,
    "the tilts in the opposite order must not be R·R′",
    expected = "is not R·R′",
    {
        for (levels, want) in wide_cases() {
            check_chain(&levels, want, Got::Reversed);
        }
    }
);

negative_control!(
    saved_chart_wide_targets_dropped,
    "a composition whose third and fourth offsets are dropped must not be R·R′",
    expected = "is not R·R′",
    {
        for (levels, want) in wide_cases() {
            check_chain(&levels, want, Got::Dropped);
        }
    }
);

// ── Refusals ─────────────────────────────────────────────────────────────────────────────────────────────────────────

/// Compositions the kernel refuses (`admitted` false) and ones it takes, `admitted` true being the control's inversion.
fn check_refused(admitted: bool) {
    let at = CENTRE;
    let corner = spec(-90.0, 90.0, TiltMap::Lean, 4, 4);
    let of = |tilts: &[TiltSpec]| {
        let levels: Vec<Level> = tilts.iter().map(|t| level(*t, at)).collect();
        Composed::of(&levels).is_some()
    };
    let two = |x, y| spec(10.0, 5.0, TiltMap::Lean, x, y);
    let refused = [
        (of(&[two(2, 3), two(4, 5), two(6, 7)]), "onto six controls"),
        (of(&[two(2, 3), two(4, 5), two(2, 6)]), "onto five controls"),
        (
            of(&[corner, spec(10.0, 0.0, TiltMap::Sheet, 4, 4)]),
            "into the target at the rank-losing corner",
        ),
        (
            of(&[two(2, 3), corner, spec(10.0, 0.0, TiltMap::Sheet, 4, 4)]),
            "into the corner's target through a chain",
        ),
        (
            of(&[two(2, 3), two(4, 5), two(6, 7), two(2, 3)]),
            "past three tilts",
        ),
    ];
    let admits = [
        (of(&[two(2, 3), two(4, 5)]), "onto four controls"),
        (
            of(&[two(2, 3), two(4, 5), two(2, 4)]),
            "a chain onto four controls",
        ),
        (
            of(&[corner, spec(10.0, 0.0, TiltMap::Sheet, 5, 5)]),
            "the corner, tilted into another target",
        ),
        (
            of(&[two(2, 3), two(2, 3), two(2, 3)]),
            "three tilts on two controls",
        ),
    ];
    for (taken, what) in refused {
        assert!(taken == admitted, "a composition {what} was not refused");
    }
    for (taken, what) in admits {
        assert!(taken != admitted, "a composition {what} was refused");
    }
}

#[test]
fn saved_chart_wide_refused() {
    check_refused(false);
}

negative_control!(
    saved_chart_wide_refused,
    "an admitted composition checked as refused must fail",
    expected = "was not refused",
    {
        check_refused(true);
    }
);

// ── A saved chart saved again ────────────────────────────────────────────────────────────────────────────────────────

/// The saved tilt, the view's tilt it is saved again with, and the new one, over three centres.
struct Tilts {
    saved: TiltSpec,
    link: TiltSpec,
    new: TiltSpec,
}

const LINK_CENTRE: [f64; 2] = [0.45, 0.52];
const NEW_CENTRE: [f64; 2] = [0.4, 0.58];

fn chain_cases() -> Vec<(Tilts, usize)> {
    vec![
        // All three into the same target: the rotations compose.
        (
            Tilts {
                saved: spec(25.0, 0.0, TiltMap::Sheet, 2, 2),
                link: spec(10.0, 14.0, TiltMap::Sheet, 2, 2),
                new: spec(-7.0, 12.0, TiltMap::Sheet, 2, 2),
            },
            1,
        ),
        // Three different targets: the block form, each offset on its own.
        (
            Tilts {
                saved: spec(25.0, 0.0, TiltMap::Sheet, 2, 2),
                link: spec(10.0, 14.0, TiltMap::Sheet, 3, 3),
                new: spec(-7.0, 12.0, TiltMap::Sheet, 4, 4),
            },
            3,
        ),
        // Two targets each, one shared down the chain: four coordinates.
        (
            Tilts {
                saved: spec(12.0, 9.0, TiltMap::Lean, 2, 3),
                link: spec(10.0, 14.0, TiltMap::Lean, 3, 4),
                new: spec(-7.0, 12.0, TiltMap::Lean, 4, 5),
            },
            4,
        ),
        // A skewed saved tilt, the next into its target.
        (
            Tilts {
                saved: spec(30.0, 40.0, TiltMap::Lean, 4, 4),
                link: spec(10.0, 0.0, TiltMap::Sheet, 4, 4),
                new: spec(8.0, 6.0, TiltMap::Lean, 4, 5),
            },
            2,
        ),
    ]
}

fn saved_chain(t: &Tilts) -> Chain {
    let saved = Saved::curved(&sphere(), t.saved, CENTRE, SPAN).expect("a curved parent");
    match saved.again(&t.link, LINK_CENTRE, SPAN) {
        Some(Saved::Chained(chain)) => chain,
        _ => panic!("a saved curved chart saves again"),
    }
}

/// A chain through its two records, read back and reopened under the new tilt.
fn reopened_chain(t: &Tilts, new: &TiltSpec) -> Tilted<ShapeSphere<f64>, f64, Composed<f64>> {
    let def = Saved::Chained(saved_chain(t));
    let link = def.step_words().expect("a link record");
    let Some(Saved::Chained(chain)) = Saved::from_chain_words(&def.to_words(), &link) else {
        panic!("the records do not read back as a chain");
    };
    chain.open(new, NEW_CENTRE, SPAN).expect("a composition")
}

/// The chain `R·R′·R″`, through its words, places every pixel as the matrices do, and names the coordinates it should.
fn check_chained(got: Got) {
    for (t, want) in chain_cases() {
        let levels = [
            level(t.saved, CENTRE),
            level(t.link, LINK_CENTRE),
            level(t.new, NEW_CENTRE),
        ];
        let view = reopened_chain(&t, &t.new);
        check_composed(*view.tilt(), &levels, want, got);
    }
}

#[test]
fn saved_chart_wide_chain() {
    check_chained(Got::Kernel);
}

negative_control!(
    saved_chart_wide_chain,
    "the chain's tilts in the opposite order must not be R·R′·R″",
    expected = "is not R·R′",
    {
        check_chained(Got::Reversed);
    }
);

/// A chain reopened at `τ″ = 0` is the view it was saved from, bit for bit: the base chart reopened under the link's
/// tilt, at every pixel, placed at f64 and f32 and decoded. With `new` another tilt (the control) it is another view.
fn check_reopens(new: TiltSpec) {
    for (t, _) in chain_cases() {
        let from = chain_view(&t);
        let again = reopened_chain(&t, &new);
        let (a, b) = (
            from.tilt().at_precision::<f32>(),
            again.tilt().at_precision::<f32>(),
        );
        for [u, v] in window() {
            let (x, y) = (
                from.tilt().place::<f64>(u, v),
                again.tilt().place::<f64>(u, v),
            );
            assert_eq!(
                (x.0.map(f64::to_bits), shift_bits(&x.1)),
                (y.0.map(f64::to_bits), shift_bits(&y.1)),
                "the reopened view is not the one saved at ({u}, {v})"
            );
            let (x, y) = (
                a.place::<f32>(u as f32, v as f32),
                b.place::<f32>(u as f32, v as f32),
            );
            assert_eq!(
                (x.0.map(f32::to_bits), x.1.dx.to_bits()),
                (y.0.map(f32::to_bits), y.1.dx.to_bits()),
                "the reopened view is not the one saved at ({u}, {v}), at f32"
            );
            assert_eq!(
                format!("{:?}", from.decode::<Defaults, f64>(u, v)),
                format!("{:?}", again.decode::<Defaults, f64>(u, v)),
                "the reopened view is not the one saved at ({u}, {v})"
            );
        }
    }
}

/// The view the chain was saved from: its base reopened under the link's tilt about the link's centre.
fn chain_view(t: &Tilts) -> Tilted<ShapeSphere<f64>, f64, Composed<f64>> {
    let Saved::Curved(base) = Saved::curved(&sphere(), t.saved, CENTRE, SPAN).expect("a parent")
    else {
        panic!("a curved parent");
    };
    base.open(&t.link, LINK_CENTRE, SPAN)
        .expect("a composition")
}

fn shift_bits(s: &Shift<f64>) -> [u64; 9] {
    [
        u64::from(s.x),
        s.dx.to_bits(),
        u64::from(s.y),
        s.dy.to_bits() & u64::from(s.two).wrapping_neg(),
        u64::from(s.two),
        u64::from(s.z),
        s.dz.to_bits(),
        u64::from(s.w),
        u64::from(s.more),
    ]
}

#[test]
fn saved_chart_wide_chain_reopens() {
    check_reopens(spec(0.0, 0.0, TiltMap::Sheet, 2, 2));
}

negative_control!(
    saved_chart_wide_chain_reopens,
    "a chain reopened under a tilt is not the view it was saved from",
    expected = "is not the one saved",
    {
        check_reopens(spec(3.0, 0.0, TiltMap::Sheet, 2, 2));
    }
);

/// The chain's two records read back bit for bit; every word of either that is changed reads as another chain or none;
/// a lone head, a lone link, the records swapped and a head of kind 3 are no chain; nothing else saves again.
fn check_chain_words(alter: bool) {
    for (t, _) in chain_cases() {
        let def = Saved::Chained(saved_chain(&t));
        let (head, link) = (def.to_words(), def.step_words().expect("a link"));
        assert_eq!(
            Saved::from_chain_words(&head, &link),
            Some(def),
            "the records read back"
        );
        assert_eq!(
            Saved::from_words(&head),
            None,
            "a lone head read as a chart"
        );
        assert_eq!(
            Saved::from_words(&link),
            None,
            "a lone link read as a chart"
        );
        assert_eq!(
            Saved::from_chain_words(&link, &head),
            None,
            "swapped records read"
        );
        let plain = Saved::curved(&sphere(), t.saved, CENTRE, SPAN).expect("a parent");
        assert_eq!(
            Saved::from_chain_words(&plain.to_words(), &link),
            None,
            "a curved head read as a chain's"
        );
        for k in 0..WORDS {
            let mut other = link;
            if alter {
                other[k] = f64::from_bits(other[k].to_bits() ^ 1);
            }
            let _ = k;
            let read = Saved::from_chain_words(&head, &other);
            assert!(
                read != Some(def),
                "the link record with word {k} changed read as the saved chain"
            );
        }
    }
    let plain = Saved::curved(&sphere(), chain_cases()[0].0.saved, CENTRE, SPAN).expect("a parent");
    let chained = plain
        .again(&chain_cases()[0].0.link, LINK_CENTRE, SPAN)
        .expect("saves again");
    assert!(
        chained
            .again(&chain_cases()[0].0.new, NEW_CENTRE, SPAN)
            .is_none(),
        "a chain saved again"
    );
    let oblique = Saved::latent(&kernel::chart::Latent {
        z0: [0.0; 8],
        q1: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        q2: [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    });
    assert!(
        oblique
            .again(&chain_cases()[0].0.link, LINK_CENTRE, SPAN)
            .is_none(),
        "an oblique chart saved again"
    );
}

#[test]
fn saved_chart_wide_chain_words() {
    check_chain_words(true);
}

negative_control!(
    saved_chart_wide_chain_words,
    "a link record left unchanged must read as the saved chain",
    expected = "read as the saved chain",
    {
        check_chain_words(false);
    }
);

// ── The composed chart's area weight and ray-degenerate flag ─────────────────────────────────────────────────────────

fn composed_view(levels: &[Level]) -> Tilted<ShapeSphere<f64>, f64, Composed<f64>> {
    Tilted::placed(sphere(), Composed::of(levels).expect("a composition")).expect("hidden targets")
}

/// The area of the parallelogram the window's unit steps map to in `(a, b, h₀ … h₇)` under `levels` (outermost first),
/// the centres not read: the root of the sum of the squared 2 × 2 minors of the two columns, from the matrices.
fn reference_gram(levels: &[Level]) -> f64 {
    let mut columns = [[0.0; 10]; 2];
    for (q, column) in columns.iter_mut().enumerate() {
        column[q] = 1.0;
        for l in levels.iter().rev() {
            let m = matrix(&l.tilt);
            let x = *column;
            for r in 0..10 {
                column[r] = (0..10).map(|k| m[r][k] * x[k]).sum();
            }
        }
    }
    let mut sum = 0.0;
    for i in 0..10 {
        for j in i + 1..10 {
            let minor = columns[0][i] * columns[1][j] - columns[0][j] * columns[1][i];
            sum += minor * minor;
        }
    }
    Float::sqrt(sum)
}

fn skewed() -> TiltSpec {
    spec(30.0, 40.0, TiltMap::Lean, 4, 4)
}

fn corner() -> TiltSpec {
    spec(-90.0, 90.0, TiltMap::Lean, 4, 4)
}

/// Over the window: with rotations only, the weight is exactly 1; with a skewed tilt it is the reference's area to
/// `64 ε` of itself (a criterion fixed before the run), inside any composition and whichever tilt is skewed.
fn check_weights(product: bool) {
    let rotations = [
        vec![
            level(spec(25.0, 15.0, TiltMap::Sheet, 2, 2), CENTRE),
            level(spec(-12.0, 18.0, TiltMap::Sheet, 2, 2), NEW_CENTRE),
        ],
        vec![
            level(spec(10.0, 5.0, TiltMap::Lean, 2, 3), CENTRE),
            level(spec(8.0, 6.0, TiltMap::Lean, 4, 5), NEW_CENTRE),
        ],
        vec![
            level(spec(25.0, 0.0, TiltMap::Sheet, 2, 2), CENTRE),
            level(spec(10.0, 14.0, TiltMap::Sheet, 3, 3), LINK_CENTRE),
            level(spec(-7.0, 12.0, TiltMap::Sheet, 2, 2), NEW_CENTRE),
        ],
    ];
    for levels in rotations {
        let view = composed_view(&levels);
        for [u, v] in window() {
            let weight = view.weight(u, v);
            assert!(
                weight == 1.0,
                "the weight of rotations at ({u}, {v}) is {weight}, not 1"
            );
        }
    }
    let bent = [
        vec![
            level(skewed(), CENTRE),
            level(spec(10.0, 0.0, TiltMap::Sheet, 4, 4), NEW_CENTRE),
        ],
        vec![
            level(skewed(), CENTRE),
            level(spec(10.0, 0.0, TiltMap::Sheet, 5, 5), NEW_CENTRE),
        ],
        vec![
            level(skewed(), CENTRE),
            level(spec(10.0, 7.0, TiltMap::Lean, 4, 5), NEW_CENTRE),
        ],
        vec![
            level(spec(20.0, 15.0, TiltMap::Sheet, 4, 4), CENTRE),
            level(skewed(), NEW_CENTRE),
        ],
        vec![
            level(skewed(), CENTRE),
            level(spec(10.0, 14.0, TiltMap::Sheet, 5, 5), LINK_CENTRE),
            level(spec(-7.0, 12.0, TiltMap::Sheet, 4, 4), NEW_CENTRE),
        ],
    ];
    for levels in bent {
        let want = reference_gram(&levels);
        let view = composed_view(&levels);
        let weight = if product {
            levels
                .iter()
                .map(|l| Tilt::new(&l.tilt, l.centre, l.span).weight)
                .product()
        } else {
            view.weight(0.5, 0.5)
        };
        eprintln!("skewed composition: weight {weight:.17} against area {want:.17}");
        assert!(
            (weight - want).abs() <= 64.0 * f64::EPSILON * want,
            "the weight with a skewed tilt is {weight}, not the area {want}"
        );
    }
}

#[test]
fn saved_chart_wide_weight() {
    check_weights(false);
}

negative_control!(
    saved_chart_wide_weight,
    "the product of each tilt's weight is not the composition's area",
    expected = "not the area",
    {
        check_weights(true);
    }
);

/// The flag and the weight at the rank-losing corner: the innermost tilt there, or the one tilt on there, is
/// `ray-degenerate` with every weight 0; an outer tilt there under an inner one on another target keeps its rank. With
/// `swap` the control reads the other tilt as the one at the corner.
fn check_corner(swap: bool) {
    let sheet = |x| spec(10.0, 0.0, TiltMap::Sheet, x, x);
    let cases = [
        (
            vec![level(corner(), CENTRE), level(sheet(4), NEW_CENTRE)],
            false,
            "the new tilt into the corner's target",
        ),
        (
            vec![level(corner(), CENTRE), level(sheet(5), NEW_CENTRE)],
            false,
            "the outer tilt at the corner",
        ),
        (
            vec![
                level(corner(), CENTRE),
                level(spec(0.0, 0.0, TiltMap::Sheet, 4, 4), NEW_CENTRE),
            ],
            true,
            "one tilt, at the corner",
        ),
        (
            vec![level(sheet(5), CENTRE), level(corner(), NEW_CENTRE)],
            true,
            "the innermost tilt at the corner",
        ),
        (
            vec![
                level(sheet(2), CENTRE),
                level(sheet(3), LINK_CENTRE),
                level(corner(), NEW_CENTRE),
            ],
            true,
            "the innermost of three at the corner",
        ),
    ];
    for (levels, degenerate, what) in cases {
        let Some(composed) = Composed::of(&levels) else {
            assert!(!degenerate, "{what}: no composition");
            continue;
        };
        let view = Tilted::placed(sphere(), composed).expect("hidden targets");
        let image = view.system_image();
        let flag = composed.rank_loss() != swap;
        assert_eq!(flag, degenerate, "{what}: the flag");
        if degenerate != swap {
            assert_eq!(image, SystemImage::RayDegenerate, "{what}: the image");
            assert!(
                window().iter().all(|&[u, v]| view.weight(u, v) == 0.0),
                "{what}: a weight"
            );
        } else {
            assert_eq!(image, SystemImage::Bijective, "{what}: the image");
            assert!(
                window().iter().all(|&[u, v]| view.weight(u, v) > 0.0),
                "{what}: a weight"
            );
        }
    }
    let kept = [level(corner(), CENTRE), level(sheet(5), NEW_CENTRE)];
    let want = reference_gram(&kept);
    let weight = composed_view(&kept).weight(0.5, 0.5);
    assert!(
        want > 0.0 && (weight - want).abs() <= 64.0 * f64::EPSILON * want,
        "the corner's area {weight} is not {want}"
    );
}

#[test]
fn saved_chart_wide_corner() {
    check_corner(false);
}

negative_control!(
    saved_chart_wide_corner,
    "the other tilt read as the one at the corner must fail",
    expected = ": the flag",
    {
        check_corner(true);
    }
);

// ── Repeat counting ──────────────────────────────────────────────────────────────────────────────────────────────────

/// The points of an `n` × `n` grid over `[lo, hi]²`, the square `[0, 1]²` and its surroundings.
fn grid(n: u32, lo: f64, hi: f64) -> Vec<[f64; 2]> {
    let at = |i: u32| lo + (hi - lo) * (f64::from(i) + 0.37) / f64::from(n);
    (0..n)
        .flat_map(|j| (0..n).map(move |i| [at(i), at(j)]))
        .collect()
}

/// How many of `points` the composition places inside the chart's square `[0, 1]²`.
fn inside(c: &Composed<f64>, points: &[[f64; 2]]) -> u64 {
    points
        .iter()
        .filter(|&&[u, v]| {
            let ([a, b], _) = c.place::<f64>(u, v);
            (0.0..=1.0).contains(&a) && (0.0..=1.0).contains(&b)
        })
        .count() as u64
}

/// With one tilt on, the composition counts, flags and weighs as that tilt alone on the chart, at every pixel of a grid
/// that runs past the square, whichever of the two places it holds; `other` is the tilt the control compares with.
fn check_one_on(other: bool) {
    let tilts = [
        spec(180.0, 0.0, TiltMap::Sheet, 2, 2),
        spec(20.0, 10.0, TiltMap::Sheet, 2, 2),
        spec(30.0, 45.0, TiltMap::Lean, 2, 2),
        spec(60.0, 60.0, TiltMap::Lean, 2, 3),
        corner(),
    ];
    let points = grid(25, -1.5, 2.5);
    for (k, t) in tilts.iter().enumerate() {
        let off = spec(0.0, 0.0, TiltMap::Sheet, 2, 2);
        let against = if other {
            tilts[(k + 1) % tilts.len()]
        } else {
            *t
        };
        let single =
            Tilted::new(sphere(), Tilt::new(&against, CENTRE, SPAN)).expect("a hidden target");
        for levels in [
            [level(*t, CENTRE), level(off, CENTRE)],
            [level(off, CENTRE), level(*t, CENTRE)],
        ] {
            let view = composed_view(&levels);
            assert_eq!(
                view.system_image(),
                single.system_image(),
                "the image of {t:?}"
            );
            assert_eq!(
                view.counted_total(points.iter().copied()),
                single.counted_total(points.iter().copied()),
                "the count of {t:?} alone is not the single tilt's"
            );
            for &[u, v] in &points {
                assert_eq!(view.pixel(u, v), single.pixel(u, v), "the pixel of {t:?}");
                assert_eq!(
                    view.weight(u, v).to_bits(),
                    single.weight(u, v).to_bits(),
                    "the weight of {t:?}"
                );
            }
        }
    }
}

#[test]
fn saved_chart_wide_one_tilt_on() {
    check_one_on(false);
}

negative_control!(
    saved_chart_wide_one_tilt_on,
    "another tilt's count is not the composition's",
    expected = "is not the single tilt's",
    {
        check_one_on(true);
    }
);

/// With several tilts on: where every tilt keeps every hidden offset (`N = 0` on each), a pixel past the square is a
/// repeat, so the count is the pixels placed inside it; where one does not, nothing is folded and every pixel counts.
/// `wrong` swaps the two expectations (the control's).
fn check_several(wrong: bool) {
    let flip = spec(180.0, 0.0, TiltMap::Sheet, 2, 2);
    let points = grid(25, -1.5, 2.5);
    let all = points.len() as u64;
    let keeps =
        Composed::of(&[level(flip, CENTRE), level(flip, NEW_CENTRE)]).expect("a composition");
    let moves = Composed::of(&[
        level(flip, CENTRE),
        level(spec(30.0, 0.0, TiltMap::Sheet, 2, 2), NEW_CENTRE),
    ])
    .expect("a composition");
    assert_eq!(
        keeps.normal_zero(),
        [true, true],
        "every tilt keeps every offset"
    );
    assert_eq!(
        moves.normal_zero(),
        [false, false],
        "one tilt moves an offset"
    );
    for (c, folded) in [(keeps, true), (moves, false)] {
        let view = Tilted::placed(sphere(), c).expect("a hidden target");
        let want = if folded != wrong {
            inside(&c, &points)
        } else {
            all
        };
        assert_eq!(
            view.counted_total(points.iter().copied()),
            want,
            "the count of a composition that {} its offsets",
            if folded { "keeps" } else { "moves" }
        );
        let outside = points.iter().find(|&&[u, v]| {
            let ([a, b], _) = c.place::<f64>(u, v);
            !(0.0..=1.0).contains(&a) || !(0.0..=1.0).contains(&b)
        });
        let &[u, v] = outside.expect("a pixel past the square");
        assert_eq!(
            view.pixel(u, v) == Some(PixelCount::System),
            folded == wrong,
            "a pixel past the square of a composition that {} its offsets",
            if folded { "keeps" } else { "moves" }
        );
    }
}

#[test]
fn saved_chart_wide_several_on() {
    check_several(false);
}

negative_control!(
    saved_chart_wide_several_on,
    "a fold where the offsets move, and none where they are kept, must fail",
    expected = "the count of a composition",
    {
        check_several(true);
    }
);

/// A placement of the identity on `(a, b)` with the hidden offsets `d`, the third and fourth read from the pixel: it
/// places exactly, which a tilt's sines and cosines do not, so a pixel lands on `r = π` of the exponential map.
/// The offsets on the four targets 2, 3, 4, 5 of the pixel `(u, v)`.
type Offsets = fn(f64, f64) -> [f64; 4];

struct Exact {
    d: Offsets,
}

impl Placement<f64> for Exact {
    fn on(&self) -> bool {
        true
    }
    fn place<R: Float + From<f64>>(&self, u: R, v: R) -> ([R; 2], Shift<R>) {
        let widen = <R as From<f64>>::from;
        let at = (self.d)(u.to_f64().unwrap_or(0.0), v.to_f64().unwrap_or(0.0));
        let [d2, d3, d4, d5] = at.map(widen);
        (
            [u, v],
            Shift {
                x: 2,
                dx: d2,
                y: 3,
                dy: d3,
                two: true,
                z: 4,
                dz: d4,
                w: 5,
                dw: d5,
                more: 2,
            },
        )
    }
    fn targets(&self) -> ([u32; 2], bool) {
        ([2, 3], true)
    }
    fn more_targets(&self) -> ([u32; 2], u32) {
        ([4, 5], 2)
    }
    fn area_weight(&self) -> f64 {
        1.0
    }
}

/// The two pixels on `r = π` of the exponential map, `(1, ½)` and `(0, ½)`, which are the one system `−n₀` where their
/// offsets agree on all four targets and two where any differs, the third and the fourth included. `ignore` expects
/// the wrong answer for a difference on the third or fourth alone (the control's: those slots unread).
fn check_antipodes(ignore: bool) {
    let mut chart = sphere();
    chart.map = ShapeMap::Exponential;
    chart.scale = core::f64::consts::PI;
    let points = [[1.0, 0.5], [0.0, 0.5]];
    let cases: [(Offsets, u64, &str); 5] = [
        (|_, _| [0.1, 0.2, 0.3, 0.4], 1, "the same offsets"),
        (|u, _| [0.1 * u, 0.2, 0.3, 0.4], 2, "the first differing"),
        (|u, _| [0.1, 0.2 * u, 0.3, 0.4], 2, "the second differing"),
        (|u, _| [0.1, 0.2, 0.3 * u, 0.4], 2, "the third differing"),
        (|u, _| [0.1, 0.2, 0.3, 0.4 * u], 2, "the fourth differing"),
    ];
    for (d, expected, what) in cases {
        let view = Tilted::placed(chart, Exact { d }).expect("hidden targets");
        let folds = Folds {
            normal_zero: [false, false],
            strip: None,
        };
        for &[u, v] in &points {
            assert_eq!(
                view.pixel_by(folds, u, v),
                Some(PixelCount::Antipode),
                "a pixel on r = π"
            );
        }
        let want = if ignore && (what.contains("third") || what.contains("fourth")) {
            1
        } else {
            expected
        };
        assert_eq!(
            view.counted_by(folds, points.iter().copied()),
            want,
            "the antipode's count with {what}"
        );
    }
}

#[test]
fn saved_chart_wide_antipodes() {
    check_antipodes(false);
}

negative_control!(
    saved_chart_wide_antipodes,
    "a count that reads only the first two offsets must fail",
    expected = "the antipode's count",
    {
        check_antipodes(true);
    }
);
