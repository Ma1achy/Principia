//! qa's tests for TASK-M2-36's second round, written from R-655 and REQ-CHART-090, each with its negative control
//! (R-176):
//!
//! - `qa_lattice_orbits` (R-655; chart_decoder_contract § "Repeats and the canonical point"; R-408): for each stored
//!   tilt with a generator, `w = (k₀, 2j₀)` is worked by hand from `N_a·k + 2·N_b·j = 0` (the ruling's ratios: `τ₁/τ₂`,
//!   `tan τ₁/tan τ₂`, `t`'s period). The window's step that moves `(a, b)` by `w` is solved from R's columns, written
//!   here from the contract's formulas, and it must leave every hidden offset fixed. Along a whole orbit every pixel
//!   decodes one system, exactly one is counted, it lies in the strip `|offset along w| ≤ |w|/2`, and a pixel in the
//!   square is that one. A unit step in `s`, no lattice vector, stays a second system, counted.
//! - `qa_lattice_where_there_is_none`: the rule's other branches, decided exactly on the stored angles. One ulp off a
//!   generator's ratio folds nothing, `N_a = 0` alone folds `s` and not `t + 2`, two targets with both offsets moving
//!   fold nothing, RQ-471's corner folds nothing, `N_a = N_b = 0` folds past the square, and the exponential map,
//!   bounded past its square, has no repeats.
//! - `qa_antipode_per_offset` (REQ-CHART-090; chart_reference §3.3 "Pixels on `r = π` under a tilt"): `counted_total`
//!   over pixels on the circle is the number of distinct `d`, in any walk order, the closed form of `d` telling which;
//!   once together at `N_a = N_b = 0` by the stored angles; no tolerance on `d` elsewhere.

use kernel::chart::shape_sphere::PixelCount;
use kernel::chart::{Chart, ShapeMap, ShapeSphere, Tilt, TiltMap, TiltSpec, Tilted};
use kernel::decode::Decoded;
use kernel::generated::links::Defaults;
use validation::negative_control;

const SPAN: [f64; 2] = [2.0, 2.0];
const CENTRE: [f64; 2] = [0.5, 0.5];
/// Two pixels of one orbit decode one system to rounding: far past the square the offsets carry `~ |w|·ε`.
const SAME: f64 = 1e-9;
/// Two systems differ by far more than `SAME`; the smallest tilt in the table moves `d` by about 0.02.
const DIFFERENT: f64 = 1e-6;

type View = Tilted<ShapeSphere<f64>, f64>;

fn spec(tau1: f64, tau2: f64, map: TiltMap, x: u32, y: u32) -> TiltSpec {
    TiltSpec {
        tau1,
        tau2,
        map,
        x,
        y,
    }
}

fn lean(t1: f64, t2: f64) -> TiltSpec {
    spec(t1, t2, TiltMap::Lean, 3, 3)
}

fn sheet(t1: f64, t2: f64) -> TiltSpec {
    spec(t1, t2, TiltMap::Sheet, 3, 3)
}

fn two(t1: f64, t2: f64) -> TiltSpec {
    spec(t1, t2, TiltMap::Lean, 2, 3)
}

fn view(map: ShapeMap, tilt: &TiltSpec, scale: f64) -> View {
    let chart = ShapeSphere {
        map,
        n0: [0.0, 0.0, 1.0],
        e1: [1.0, 0.0, 0.0],
        e2: [0.0, 1.0, 0.0],
        scale,
        inertia: 1.0,
        phase: 0.3,
        z_mass: [-0.6, 0.25],
        z_mom: [0.1, 0.35, -0.2, -0.05],
    };
    Tilted::new(chart, Tilt::new(tilt, CENTRE, SPAN)).unwrap()
}

fn spherical(tilt: &TiltSpec) -> View {
    view(ShapeMap::Spherical, tilt, 1.0)
}

/// `R·e_a` and `R·e_b` in `(a, b, d_X, d_Y)`, from the contract's words (§ "Tilt (basis edit)"): Rodrigues' turn for
/// Tip as sheet with one target and both angles set, `(cos τ, sin τ)` per axis otherwise.
fn columns(s: &TiltSpec) -> [[f64; 4]; 2] {
    let rad = |d: f64| d.to_radians().sin_cos();
    if s.map == TiltMap::Sheet && s.x == s.y && s.tau1 != 0.0 && s.tau2 != 0.0 {
        let m = s.tau1.hypot(s.tau2);
        let (ka, kb) = (s.tau2 / m, -s.tau1 / m);
        let (sm, cm) = rad(m);
        let w = 1.0 - cm;
        return [
            [cm + ka * ka * w, ka * kb * w, -kb * sm, 0.0],
            [ka * kb * w, cm + kb * kb * w, ka * sm, 0.0],
        ];
    }
    let ((s1, c1), (s2, c2)) = (rad(s.tau1), rad(s.tau2));
    if s.x == s.y {
        [[c1, 0.0, s1, 0.0], [0.0, c2, s2, 0.0]]
    } else {
        [[c1, 0.0, s1, 0.0], [0.0, c2, 0.0, s2]]
    }
}

/// The window step `(Δu, Δv)` that best moves `(a, b)` by `w` with no hidden offset moving (least squares on R's
/// columns); the caller checks through the view that it does so exactly.
fn step(s: &TiltSpec, w: [f64; 2]) -> [f64; 2] {
    let [cu, cv] = columns(s);
    let dot = |x: [f64; 4], y: [f64; 4]| (0..4).map(|i| x[i] * y[i]).sum::<f64>();
    let target = [w[0], w[1], 0.0, 0.0];
    let (g11, g12, g22) = (dot(cu, cu), dot(cu, cv), dot(cv, cv));
    let (r1, r2) = (dot(cu, target), dot(cv, target));
    let det = g11 * g22 - g12 * g12;
    [(r1 * g22 - r2 * g12) / det, (r2 * g11 - r1 * g12) / det]
}

fn flat(d: &Decoded<f64>) -> Vec<f64> {
    let mut v = d.ic.m.to_vec();
    v.extend(d.ic.r.iter().flatten());
    v.extend(d.ic.p.iter().flatten());
    v
}

fn gap(v: &View, p: [f64; 2], q: [f64; 2]) -> f64 {
    let x = flat(&v.decode::<Defaults, f64>(p[0], p[1]));
    let y = flat(&v.decode::<Defaults, f64>(q[0], q[1]));
    x.iter()
        .zip(y)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f64::max)
}

fn offsets(v: &View, p: [f64; 2]) -> [f64; 2] {
    let shift = v.at(p[0], p[1]).1.unwrap();
    [shift.dx, shift.dy]
}

/// Whether `p` is counted: by the kernel, or under `no_lattice` by R-554 L3's rule alone (the control).
fn counted(v: &View, p: [f64; 2], no_lattice: bool) -> bool {
    if no_lattice {
        let [a, b] = v.at(p[0], p[1]).0;
        let inside = |x: f64| (0.0..=1.0).contains(&x);
        let [na, nb] = v.tilt().normal_zero;
        !((!inside(a) && na) || (!inside(b) && na && nb))
    } else {
        v.pixel(p[0], p[1]) == Some(PixelCount::System)
    }
}

/// The hand-worked generators (R-655's cases), `w = (k₀, 2j₀)`.
fn generators() -> Vec<(TiltSpec, [f64; 2])> {
    vec![
        // N_b = 0 alone: t's period.
        (sheet(30.0, 0.0), [0.0, 2.0]),
        (lean(30.0, 0.0), [0.0, 2.0]),
        (lean(90.0, 30.0), [0.0, 2.0]),
        (two(30.0, 0.0), [0.0, 2.0]),
        (two(30.0, 180.0), [0.0, 2.0]),
        // Lean, tan τ₁/tan τ₂ = ±1: k + 2j·(±1) = 0.
        (lean(45.0, 45.0), [2.0, -2.0]),
        (lean(-45.0, -45.0), [2.0, -2.0]),
        (lean(45.0, -45.0), [2.0, 2.0]),
        (lean(30.0, -30.0), [2.0, 2.0]),
        // Lean, the ratio ±1/3: k/3 ± 2j = 0, so (6, ∓2).
        (lean(30.0, 60.0), [6.0, -2.0]),
        (lean(-30.0, 60.0), [6.0, 2.0]),
        // Lean, the ratio ±3: 3k ± 2j = 0, so (2, ∓6).
        (lean(60.0, 30.0), [2.0, -6.0]),
        (lean(-60.0, 30.0), [2.0, 6.0]),
        // Tip as sheet, τ₁·k + 2·τ₂·j = 0: 30k + 40j = 0, (4, −6); 40k + 140j = 0, (7, −4); ½k + ½j = 0, (1, −2).
        (sheet(30.0, 20.0), [4.0, -6.0]),
        (sheet(-30.0, 20.0), [4.0, 6.0]),
        (sheet(40.0, 70.0), [7.0, -4.0]),
        (sheet(0.5, 0.25), [1.0, -2.0]),
        // Past m = 180° (wrapped, the same plane): 170k − 120j = 0, (12, 34).
        (sheet(170.0, -60.0), [12.0, 34.0]),
    ]
}

/// Base pixels over and past the square, on a grid no strip's edge falls on.
fn bases() -> Vec<[f64; 2]> {
    let at = |k: u32| -0.513 + 2.083 * f64::from(k) / 6.0;
    (0..7)
        .flat_map(|j| (0..7).map(move |i| [at(i), at(j) + 0.0071]))
        .collect()
}

/// R-655 along whole orbits. `no_lattice` counts with R-554 L3's rule alone (the control).
fn check_orbits(no_lattice: bool) {
    for (s, w) in generators() {
        let v = spherical(&s);
        let [du, dv] = step(&s, w);
        let len = w[0].hypot(w[1]);
        for base in bases() {
            let at = |n: i32| [base[0] + f64::from(n) * du, base[1] + f64::from(n) * dv];
            let [a0, b0] = v.at(base[0], base[1]).0;
            let d0 = offsets(&v, base);
            let mut once = 0;
            for n in -5..=5 {
                let p = at(n);
                let [a, b] = v.at(p[0], p[1]).0;
                let m = f64::from(n);
                assert!(
                    (a - a0 - m * w[0]).abs() < SAME && (b - b0 - m * w[1]).abs() < SAME,
                    "{s:?}: the step does not move (a, b) by {n}·{w:?}"
                );
                let d = offsets(&v, p);
                assert!(
                    (d[0] - d0[0]).abs() < SAME && (d[1] - d0[1]).abs() < SAME,
                    "{s:?}: {w:?} moves a hidden offset, {d:?} from {d0:?}"
                );
                let far = gap(&v, base, p);
                assert!(far < SAME, "{s:?}: {base:?} and {p:?} differ by {far}");
                if counted(&v, p, no_lattice) {
                    once += 1;
                    let along = ((a - 0.5) * w[0] + (b - 0.5) * w[1]) / len;
                    assert!(
                        along.abs() <= len / 2.0 + SAME,
                        "{s:?}: counted at ({a}, {b}), outside the strip"
                    );
                }
            }
            assert_eq!(once, 1, "{s:?}: the orbit of {base:?} counted {once} times");
            if (0.0..=1.0).contains(&a0) && (0.0..=1.0).contains(&b0) {
                assert!(
                    counted(&v, base, no_lattice),
                    "{s:?}: a pixel in the square is not canonical"
                );
            }
        }
        // A unit step in s, no lattice vector, where the plane lets a move: a second system, counted.
        let [cu, cv] = columns(&s);
        let det = cu[0] * cv[1] - cu[1] * cv[0];
        if det.abs() > 1e-9 {
            let q = [0.5 + cv[1] / det, 0.5 - cv[0] / det];
            let far = gap(&v, CENTRE, q);
            assert!(far > DIFFERENT, "{s:?}: s + 1 decodes the same system");
            assert!(counted(&v, CENTRE, false) && counted(&v, q, false), "{s:?}");
        }
    }
}

#[test]
fn qa_lattice_orbits() {
    check_orbits(false);
}

negative_control!(
    qa_lattice_orbits,
    "R-554 L3's rule without the lattice counts an orbit many times, so it must fail",
    expected = "counted",
    check_orbits(true)
);

/// The branches with no generator. `tolerance` folds where `N_a·k + 2·N_b·j` of a computed `N` is under `1e-12` (the
/// control): the ruling's "never by a tolerance on a computed `N`".
fn check_none(tolerance: bool) {
    // One ulp off the ratio: a lattice whose generator lies past any view, so both ends count.
    for (s, w) in [
        (lean(45.0, f64::next_up(45.0)), [2.0, -2.0]),
        (lean(30.0, f64::next_up(60.0)), [6.0, -2.0]),
        (sheet(30.0, f64::next_up(20.0)), [4.0, -6.0]),
    ] {
        let v = spherical(&s);
        let [du, dv] = step(&s, w);
        let q = [0.5 + du, 0.5 + dv];
        let [cu, cv] = columns(&s);
        let n = [cu[1] * cv[2] - cu[2] * cv[1], cu[2] * cv[0] - cu[0] * cv[2]];
        let near = (n[0] * w[0] + n[1] * w[1]).abs() < 1e-12;
        assert!(near, "{s:?}: the computed N is one rounding from a lattice");
        let both = counted(&v, CENTRE, false) && counted(&v, q, false) && !(tolerance && near);
        assert!(both, "{s:?}: a pair one ulp off the lattice folded");
    }
    // N_a = 0 alone: θ's period folds, the canonical s in [0, 1]; t + 2 moves d, so it is another system.
    for s in [lean(30.0, 90.0), sheet(0.0, 30.0), lean(0.0, 30.0)] {
        let v = spherical(&s);
        let [_, cv] = columns(&s);
        assert_eq!(v.pixel(0.5, 0.5), Some(PixelCount::System), "{s:?}");
        let [a, _] = v.at(1.5, 0.5).0;
        assert!(a > 1.0, "{s:?}: past s");
        assert_eq!(v.pixel(1.5, 0.5), None, "{s:?}: s + 1 is a repeat");
        let q = [0.5, 0.5 + 2.0 / cv[1]];
        assert_eq!(v.pixel(q[0], q[1]), Some(PixelCount::System), "{s:?}");
        assert!(gap(&v, CENTRE, q) > DIFFERENT, "{s:?}: t + 2 keeps d");
    }
    // Two targets, both offsets moving: d_X with a and d_Y with b, so no integer pair keeps both.
    for s in [two(45.0, 45.0), two(30.0, 60.0)] {
        let v = spherical(&s);
        let [cu, cv] = columns(&s);
        let q = [0.5 + 2.0 / cu[0], 0.5 - 2.0 / cv[1]];
        assert_eq!(s.strip(), None, "{s:?}");
        assert_eq!(v.pixel(0.5, 0.5), Some(PixelCount::System), "{s:?}");
        assert_eq!(v.pixel(q[0], q[1]), Some(PixelCount::System), "{s:?}");
        assert!(gap(&v, CENTRE, q) > DIFFERENT, "{s:?}");
    }
    // RQ-471's corner: no fold at all, anywhere in or past the square.
    for s in [lean(90.0, 90.0), lean(-90.0, 90.0)] {
        let v = spherical(&s);
        for [p, q] in bases() {
            let far = [p * 40.0, q * 40.0];
            assert_eq!(v.pixel(p, q), Some(PixelCount::System), "{s:?}");
            assert_eq!(v.pixel(far[0], far[1]), Some(PixelCount::System), "{s:?}");
        }
    }
    // N_a = N_b = 0: the pole maps and both periods, as R-554 L3 has it.
    let v = spherical(&sheet(108.0, 144.0));
    assert_eq!(v.pixel(0.5, 0.5), Some(PixelCount::System));
    assert_eq!(v.pixel(0.5, 8.0), None);
    assert_eq!(v.pixel(8.0, 0.5), None);
    // The exponential map, bounded past its square, has no repeats: its pixels are as they were.
    for (s, _) in generators() {
        let v = view(ShapeMap::Exponential, &s, 1.0);
        for [u, w] in bases() {
            let [a, b] = v.at(u, w).0;
            let past = !(0.0..=1.0).contains(&a) || !(0.0..=1.0).contains(&b);
            let got = v.pixel(u, w);
            if past {
                assert_eq!(got, Some(PixelCount::DomainEnd), "{s:?}: past the square");
            } else {
                assert_ne!(got, None, "{s:?}: a repeat inside the square");
            }
        }
    }
}

#[test]
fn qa_lattice_where_there_is_none() {
    check_none(false);
}

negative_control!(
    qa_lattice_where_there_is_none,
    "a tolerance on the computed N folds a pair one ulp off the lattice, so it must fail",
    expected = "one ulp off the lattice",
    check_none(true)
);

/// The tally under test: the kernel's, or a control's.
#[derive(Clone, Copy)]
struct Tally {
    /// One boolean across the view, the count before REQ-CHART-090.
    shared: bool,
    /// `d` compared with a tolerance, which §3.3 forbids.
    tolerant: bool,
}

fn tally(v: &View, points: &[[f64; 2]], how: Tally) -> u64 {
    let on = |p: &[f64; 2]| v.pixel(p[0], p[1]);
    let systems = points
        .iter()
        .filter(|p| on(p) == Some(PixelCount::System))
        .count() as u64;
    let circle: Vec<[f64; 2]> = points
        .iter()
        .filter(|p| on(p) == Some(PixelCount::Antipode))
        .copied()
        .collect();
    if how.shared {
        return systems + u64::from(!circle.is_empty());
    }
    if how.tolerant {
        let mut kept: Vec<[f64; 2]> = Vec::new();
        for p in &circle {
            let d = offsets(v, *p);
            let close = |e: &[f64; 2]| (e[0] - d[0]).abs() + (e[1] - d[1]).abs() < 1e-12;
            if !kept.iter().any(close) {
                kept.push(d);
            }
        }
        return systems + kept.len() as u64;
    }
    v.counted_total(points.iter().copied())
}

fn wide(tilt: &TiltSpec) -> View {
    view(ShapeMap::Exponential, tilt, std::f64::consts::PI)
}

fn assert_on_circle(v: &View, points: &[[f64; 2]]) {
    for p in points {
        assert_eq!(
            v.pixel(p[0], p[1]),
            Some(PixelCount::Antipode),
            "{p:?} is not on r = π"
        );
    }
}

/// REQ-CHART-090, from §3.3's paragraph.
fn check_antipode(how: Tally) {
    // τ₁ = ±90°: the top and bottom edges' centres, for every u, with d = span·sin τ₁·(u − ½) by the closed form.
    for sign in [1.0, -1.0] {
        let v = wide(&sheet(sign * 90.0, 0.0));
        let us: Vec<f64> = (0..9).map(|k| 0.1 * f64::from(k) + 0.1).collect();
        let top: Vec<[f64; 2]> = us.iter().map(|&u| [u, 1.0]).collect();
        let bottom: Vec<[f64; 2]> = us.iter().map(|&u| [u, 0.0]).collect();
        assert_on_circle(&v, &top);
        assert_on_circle(&v, &bottom);
        for p in &top {
            let want = sign * 2.0 * (p[0] - 0.5);
            assert!((offsets(&v, *p)[0] - want).abs() < 1e-15, "d at {p:?}");
        }
        // Nine values of d, so nine systems, on the top alone and with the bottom's nine chords.
        assert_eq!(tally(&v, &top, how), 9, "nine d on the circle");
        let mut all: Vec<[f64; 2]> = top.iter().chain(bottom.iter()).copied().collect();
        assert_eq!(tally(&v, &all, how), 9, "nine chords, nine d");
        // Any walk order: the number of classes of an equivalence.
        all.reverse();
        assert_eq!(tally(&v, &all, how), 9, "reversed");
        all.rotate_left(5);
        assert_eq!(tally(&v, &all, how), 9, "rotated");
        // A system pixel adds itself and hides no pixel on the circle; a pixel past the circle adds nothing.
        let mut mixed = vec![[0.25, 0.5], [0.25, 1.5]];
        mixed.extend_from_slice(&top);
        assert_eq!(v.pixel(0.25, 1.5), Some(PixelCount::DomainEnd));
        assert_eq!(tally(&v, &mixed, how), 10, "a system beside nine d");
    }
    // N_a = N_b = 0 by the stored angles, under each form: one together, whatever rounding leaves in d.
    for s in [
        sheet(180.0, 0.0),
        sheet(-180.0, 0.0),
        sheet(0.0, 180.0),
        lean(180.0, 180.0),
        two(180.0, -180.0),
    ] {
        let v = wide(&s);
        assert_eq!(v.tilt().normal_zero, [true, true], "{s:?}");
        let sides = [[0.0, 0.5], [1.0, 0.5], [0.5, 0.0], [0.5, 1.0]];
        assert_on_circle(&v, &sides);
        assert_eq!(tally(&v, &sides, how), 1, "{s:?}: d unchanged");
    }
    // Elsewhere d is compared as computed, with no tolerance (the physics review's case): N_a = 0 alone, the same d_Y and
    // a d_X of rounding, two values.
    let v = wide(&two(180.0, 90.0));
    let pair = [[0.0, 0.75], [1.0, 0.75]];
    assert_on_circle(&v, &pair);
    assert_ne!(offsets(&v, pair[0])[0], offsets(&v, pair[1])[0]);
    assert_eq!(offsets(&v, pair[0])[1], offsets(&v, pair[1])[1]);
    assert_eq!(tally(&v, &pair, how), 2, "no tolerance on d");
}

#[test]
fn qa_antipode_per_offset() {
    check_antipode(Tally {
        shared: false,
        tolerant: false,
    });
}

negative_control!(
    qa_antipode_per_offset,
    "one boolean shared across the view counts nine systems as one, so it must fail",
    expected = "nine d on the circle",
    check_antipode(Tally {
        shared: true,
        tolerant: false
    })
);

mod tolerant {
    use super::*;
    negative_control!(
        qa_antipode_per_offset,
        "a tolerance on d merges offsets §3.3 compares exactly, so it must fail",
        expected = "no tolerance on d",
        check_antipode(Tally {
            shared: false,
            tolerant: true
        })
    );
}
