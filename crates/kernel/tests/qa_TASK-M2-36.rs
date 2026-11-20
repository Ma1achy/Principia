//! qa's tests for TASK-M2-36, written from the three requirements it closes, each with its negative control (R-176):
//!
//! - `qa_tilt_mass_logits` (REQ-CHART-063; R-473 T3, J8; R-477 C7): "on the shape-sphere chart tilted into each mass
//!   logit at τ ∈ {15°, 45°, 89°, 135°}, over a 65 × 65 grid every pixel has m₁, m₂, m₃ > 0 and m₁ + m₂ + m₃ = 1 to
//!   4 ulp; control: the same tilt added to the physical mass m₁ yields a pixel with a non-positive mass or a sum ≠ 1".
//!   Run on both shape maps, as `τ₁` alone, as `τ₂` alone and with both angles set under each tilt map, at the
//!   Slice centre slider's span and at a span that drives the logits deep into the link's saturation. Beside the
//!   simplex, each single-angle pixel's masses are the mass link's at the logits with `d` added, where `d` is
//!   `span · sin τ · (offset from the centre)` (chart_decoder_contract § "Tilt (basis edit)": `d` added to the hidden
//!   coordinate at the slice centre, a mass in its logit); a second control adds `d` to the other logit.
//! - `qa_tilt_units` (REQ-CHART-064; R-507 U1, U2): on the shape sphere tilted 30° into a momentum control, the tilted
//!   axis's direction in the chart's parameters (`d` per unit of `a`) is bit-identical at the window's edge at zoom 1
//!   and at zoom 4, it is `span · tan τ`, and the decode at those pixels is the chart's own at `a = c + cos τ · ũ` with
//!   the control moved by `span · sin τ · ũ` (`ũ` in units of the axis's full range at zoom 1, the chart's `[0, 1]`);
//!   untilted, on each curved chart and under both maps, the decode is bit-identical to the chart's; on the latent
//!   chart the tilted basis is the basis edit `σ(cos τ e_a + sin τ e_X)` bitwise, one target or two. Controls: a scale
//!   in window half-widths changes with zoom; a latent tilt scaled by the slider span differs from the basis edit; a
//!   tilt one hair from zero is not bitwise the chart's (the untilted check can fail).
//! - `qa_tilted_extension` (REQ-CHART-066; R-472 X3, X3a; R-508 V2; chart_reference §5.4's table): the spherical shape
//!   sphere tilted 30° into a momentum control, a pixel past the square decodes as the chart at the tilted `(a, b)`,
//!   bitwise, and that point is the pole-crossing (φ) and periodic (θ) image inside the square; both invariant charts
//!   tilted into a mass logit are not a system past `s`'s edges or below `t = 0` and are the continuing side's system
//!   past `t = 1`, classified on `(a, b)` from the closed form of R (single angles) and from Rodrigues' formula (Tip as
//!   sheet, both angles); a saved oblique chart continues affinely. Controls: the types applied to `(u, v)` before the
//!   tilt (twice: the pole crossing, and the invariant chart's bounded edges), and a per-chart table bounded on every
//!   side.

use kernel::chart::invariant::Invariant;
use kernel::chart::tilt::{latent_basis, Hidden, LatentFrame};
use kernel::chart::{
    Chart, InvariantLE, InvariantLK, Latent, ShapeMap, ShapeSphere, Tilt, TiltMap, TiltSpec,
    Tilted, ValidationResult,
};
use kernel::decode::{mass, Decoded};
use kernel::generated::links::Defaults;
use validation::negative_control;

const EPS: f64 = f64::EPSILON;

/// `(sin τ, cos τ)` for `τ` in degrees.
fn sc(deg: f64) -> (f64, f64) {
    deg.to_radians().sin_cos()
}

fn spec(tau1: f64, tau2: f64, map: TiltMap, x: u32, y: u32) -> TiltSpec {
    TiltSpec {
        tau1,
        tau2,
        map,
        x,
        y,
    }
}

/// The pixel centres of an `n` × `n` grid over `[lo, hi]²`.
fn grid(n: u32, lo: f64, hi: f64) -> Vec<[f64; 2]> {
    let at = |k: u32| lo + (hi - lo) * (f64::from(k) + 0.5) / f64::from(n);
    (0..n)
        .flat_map(|j| (0..n).map(move |i| [at(i), at(j)]))
        .collect()
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

fn invariant() -> Invariant<f64> {
    Invariant {
        z_cfg: [0.2, -0.3],
        z_mass: [0.4, -0.2],
        k_max: 1.0,
        gamma_k: 1.5,
    }
}

/// The held value of control `k` of `chart`, read from `Φ`'s output.
fn held<C: Chart<Param = f64>>(chart: &C, k: u32) -> f64 {
    chart.map(0.5, 0.5).z.z[k as usize]
}

/// Tip as sheet's `R·e_a`, `R·e_b` in `(a, b, d)`, one target, both angles set, from the contract's words: the plane
/// turned by `m = √(τ₁² + τ₂²)` about `k = (τ₂ e_a − τ₁ e_b)/m`, Rodrigues' `v cos m + (k × v) sin m + k (k·v)(1 − cos m)`.
fn rodrigues(t1: f64, t2: f64) -> [[f64; 3]; 2] {
    let m = (t1 * t1 + t2 * t2).sqrt();
    let k = [t2 / m, -t1 / m, 0.0];
    let (s, c) = sc(m);
    let rot = |v: [f64; 3]| {
        let cross = [
            k[1] * v[2] - k[2] * v[1],
            k[2] * v[0] - k[0] * v[2],
            k[0] * v[1] - k[1] * v[0],
        ];
        let dot = k[0] * v[0] + k[1] * v[1] + k[2] * v[2];
        std::array::from_fn(|i| v[i] * c + cross[i] * s + k[i] * dot * (1.0 - c))
    };
    [rot([1.0, 0.0, 0.0]), rot([0.0, 1.0, 0.0])]
}

// ── REQ-CHART-063: a mass is tilted in its logit ──────────────────────────────────────────────────────────────────────

/// `d`'s closed form, one angle set: `sin τ` and the offset `(ũ, ṽ)` from the centre give `d / span`.
type ClosedD = fn(f64, f64, f64) -> f64;

/// The mass tilt over the grid: `d` expected on the logit the tilt targets. `physical` puts `d` on the physical mass
/// `m₁` instead (the verify's control); `other` expects it on the other logit (a second control).
fn check_mass_logits(physical: bool, other: bool) {
    let centre = [0.43, 0.58];
    for shape in [ShapeMap::Spherical, ShapeMap::Exponential] {
        let chart = sphere(shape);
        let untilted = chart.decode::<Defaults, f64>(0.5, 0.5).masses;
        for x in [6u32, 7] {
            for span in [2.0, 16.0] {
                for tau in [15.0, 45.0, 89.0, 135.0] {
                    let (s, _) = sc(tau);
                    // (spec, d's closed form at the offset (ũ, ṽ) where the closed form is exact: one angle set.)
                    let cases: [(TiltSpec, Option<ClosedD>); 4] = [
                        (
                            spec(tau, 0.0, TiltMap::Sheet, x, x),
                            Some(|s, du, _dv| s * du),
                        ),
                        (
                            spec(0.0, tau, TiltMap::Lean, x, x),
                            Some(|s, _du, dv| s * dv),
                        ),
                        (spec(tau, -0.5 * tau, TiltMap::Sheet, x, x), None),
                        (spec(-0.5 * tau, tau, TiltMap::Lean, x, x), None),
                    ];
                    for (sp, closed) in cases {
                        let view = Tilted::new(chart, Tilt::new(&sp, centre, [span, span]))
                            .expect("a mass logit is a hidden coordinate of the shape sphere");
                        for [u, v] in grid(65, 0.0, 1.0) {
                            let got = view.decode::<Defaults, f64>(u, v);
                            let mut m = got.ic.m;
                            if let Some(f) = closed {
                                let d = span * f(s, u - centre[0], v - centre[1]);
                                let mut z = [chart.z_mass[0], chart.z_mass[1]];
                                if physical {
                                    m = untilted.m;
                                    m[0] += d;
                                } else {
                                    let k = if other { 7 - x } else { x - 6 };
                                    z[k as usize] += d;
                                    assert!(
                                        got.masses == mass::decode::<Defaults, f64>(z),
                                        "masses at ({u}, {v}) under {sp:?} are not the link's at the tilted logit"
                                    );
                                }
                            }
                            let sum = m[0] + m[1] + m[2];
                            assert!(
                                m.iter().all(|&mi| mi > 0.0) && (sum - 1.0).abs() <= 4.0 * EPS,
                                "masses {m:?} at ({u}, {v}) under {sp:?}, span {span}, are off the simplex"
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn qa_tilt_mass_logits() {
    check_mass_logits(false, false);
}

negative_control!(
    qa_tilt_mass_logits,
    "the same tilt added to the physical mass m₁ leaves the simplex, so it must fail",
    expected = "are off the simplex",
    check_mass_logits(true, false)
);

mod other_logit {
    use super::*;
    negative_control!(
        qa_tilt_mass_logits,
        "d added to the logit the tilt does not target is not the decode's mass, so it must fail",
        expected = "are not the link's at the tilted logit",
        check_mass_logits(false, true)
    );
}

// ── REQ-CHART-064: units that do not depend on zoom (R-507) ───────────────────────────────────────────────────────────

const SPAN: f64 = 2.0;

/// The direction `d` per unit of `a` of the 30° tilted horizontal axis at the window's right edge, at zoom 1 and zoom
/// 4, and the decode there. `half_width` reads the screen axis in units of the window's half-width, `1/(2·zoom)`, the
/// withdrawn proposal (the control).
fn check_units(half_width: bool) {
    let chart = sphere(ShapeMap::Spherical);
    let (x, tau) = (2u32, 30.0);
    let (s, c) = sc(tau);
    let tilt = Tilt::new(
        &spec(tau, 0.0, TiltMap::Sheet, x, x),
        [0.0, 0.5],
        [SPAN, SPAN],
    );
    let direction = |zoom: f64| {
        // ũ at the window's edge, in units of the axis's full range at zoom 1 (the chart's [0, 1]).
        let edge = 0.5 / zoom;
        let [da, db, d, _] = tilt.offsets(edge, 0.0);
        assert_eq!(db, 0.0, "τ₁ alone moves b");
        assert_eq!(da, c * edge, "Δa is not cos τ · ũ");
        if half_width {
            let half = 0.5 / zoom;
            // ã = ũ / half, rotated, scaled back by half; d̃ scaled by the span only.
            let a_tilde = edge / half;
            (SPAN * (s * a_tilde)) / (half * (c * a_tilde))
        } else {
            assert_eq!(d, SPAN * (s * edge), "d is not span · sin τ · ũ");
            d / da
        }
    };
    let (one, four) = (direction(1.0), direction(4.0));
    assert!(
        one.to_bits() == four.to_bits(),
        "the tilted axis's direction depends on zoom: {one} at zoom 1, {four} at zoom 4"
    );
    assert!(
        (one - SPAN * (s / c)).abs() <= 4.0 * EPS * one,
        "the direction {one} is not span · tan τ"
    );
    // The decode at those pixels, the view centred at (0.4, 0.55): the chart at (c_u + cos τ · ũ, v), the control
    // moved by span · sin τ · ũ.
    let centre = [0.4, 0.55];
    let view = Tilted::new(
        chart,
        Tilt::new(&spec(tau, 0.0, TiltMap::Sheet, x, x), centre, [SPAN, SPAN]),
    )
    .unwrap();
    for zoom in [1.0, 4.0] {
        for side in [-1.0, 1.0] {
            let u = centre[0] + side * 0.5 / zoom;
            let du = u - centre[0];
            let moved = chart.sliced(x, held(&chart, x) + SPAN * (s * du)).unwrap();
            assert_eq!(
                bits(&view.decode::<Defaults, f64>(u, centre[1])),
                bits(&moved.decode::<Defaults, f64>(centre[0] + c * du, centre[1])),
                "the edge pixel at zoom {zoom} is not the chart at the tilted point"
            );
        }
    }
}

/// At `τ` on each curved chart under both maps, the decode, `Φ` and `validate` at every pixel of a 33 × 33 grid are
/// bitwise the untilted chart's.
fn check_untilted(tau: f64) {
    fn one<C: kernel::chart::tilt::Tiltable<Param = f64> + Copy>(chart: C, x: u32, tau: f64) {
        for map in [TiltMap::Sheet, TiltMap::Lean] {
            let view = Tilted::new(
                chart,
                Tilt::new(&spec(tau, tau, map, x, x), [0.3, 0.7], [SPAN, SPAN]),
            )
            .unwrap();
            for [u, v] in grid(33, 0.0, 1.0) {
                assert!(
                    bits(&view.decode::<Defaults, f64>(u, v))
                        == bits(&chart.decode::<Defaults, f64>(u, v))
                        && format!("{:?}", view.map::<f64>(u, v))
                            == format!("{:?}", chart.map::<f64>(u, v))
                        && view.validate(u, v) == chart.validate(u, v),
                    "{} at ({u}, {v}) untilted is not bitwise the chart's",
                    chart.name()
                );
            }
        }
    }
    one(sphere(ShapeMap::Spherical), 4, tau);
    one(sphere(ShapeMap::Exponential), 7, tau);
    one(InvariantLE(invariant()), 1, tau);
    one(InvariantLK(invariant()), 6, tau);
    one(
        kernel::chart::Mixed::<kernel::chart::mixed::Alpha, kernel::chart::mixed::PRhoX, f64>::new(
            [0.1, -0.2, 0.3, -0.4, 0.2, 0.1, 0.5, -0.3],
            [0.05, 1.5],
            [-1.8, 1.8],
        ),
        5,
        tau,
    );
}

/// The latent frame: the plane `z_2 × z_3`, scale `σ`, no turn.
fn frame(scale: f64) -> LatentFrame {
    LatentFrame {
        a: 2,
        b: 3,
        scale,
        gamma: 0.0,
    }
}

/// The latent chart's tilted basis against the basis edit, `σ(cos τ₁ e_a + sin τ₁ e_X)`, `σ(cos τ₂ e_b + sin τ₂ e_Y)`
/// (Lean each axis's formula per axis, which is also the two-target form). `scaled` expects the hidden direction in
/// units of a slider span, as on a curved chart (the control).
fn check_latent(scaled: bool) {
    let unit = if scaled { SPAN } else { 1.0 };
    for scale in [0.5, 2.0] {
        for (t1, t2, map, x, y) in [
            (30.0, 0.0, TiltMap::Sheet, 4u32, 4u32),
            (0.0, -40.0, TiltMap::Lean, 6, 6),
            (30.0, 50.0, TiltMap::Lean, 5, 5),
            (30.0, 50.0, TiltMap::Sheet, 4, 7),
            (-70.0, 20.0, TiltMap::Lean, 0, 1),
        ] {
            let (s1, c1) = sc(t1);
            let (s2, c2) = sc(t2);
            let mut q1 = [0.0; 8];
            let mut q2 = [0.0; 8];
            q1[2] = scale * c1;
            q1[x as usize] = scale * (unit * s1);
            q2[3] = scale * c2;
            q2[y as usize] = scale * (unit * s2);
            let got =
                latent_basis(&frame(scale), &spec(t1, t2, map, x, y)).expect("hidden targets");
            assert!(
                got == (q1, q2),
                "the latent tilt at ({t1}, {t2}) into ({x}, {y}) is not the basis edit: {got:?}"
            );
        }
    }
}

#[test]
fn qa_tilt_units() {
    check_units(false);
    check_untilted(0.0);
    check_latent(false);
}

negative_control!(
    qa_tilt_units,
    "a scale in window half-widths changes with zoom, so it must fail",
    expected = "depends on zoom",
    check_units(true)
);

mod latent_scaled {
    use super::*;
    negative_control!(
        qa_tilt_units,
        "a latent tilt scaled by the slider span is not the basis edit, so it must fail",
        expected = "is not the basis edit",
        check_latent(true)
    );
}

mod hair {
    use super::*;
    negative_control!(
        qa_tilt_units,
        "a tilt of 1e-6 degrees is not the untilted chart, so the bitwise check must fail",
        expected = "untilted is not bitwise the chart's",
        check_untilted(1e-6)
    );
}

// ── REQ-CHART-066: the extension acts on the tilted (a, b) ────────────────────────────────────────────────────────────

/// chart_reference §5.4's spherical row: θ (horizontal) periodic, φ (vertical) pole-crossing with θ shifted by half
/// its period. The image inside the square of the chart point `(a, b)`.
fn sphere_image(a: f64, b: f64) -> (f64, f64) {
    let (a, b) = if b > 1.0 {
        (a + 0.5, 2.0 - b)
    } else if b < 0.0 {
        (a + 0.5, -b)
    } else {
        (a, b)
    };
    (a.rem_euclid(1.0), b)
}

/// The spherical shape sphere tilted 30° into a momentum control, as `τ₁` and as `τ₂`, centred near the top-right
/// corner: each pixel past the square decodes bitwise as the chart at `(a, b)`, the control moved by `d`, and the point
/// there is the one at the periodic and pole-crossing image inside the square. `before` applies the types to `(u, v)`
/// first and tilts what they give (the control).
fn check_pole(before: bool) {
    let chart = sphere(ShapeMap::Spherical);
    let x = 3u32;
    let (s, c) = sc(30.0);
    let centre = [0.8, 0.85];
    let mut past = 0;
    for (t1, t2) in [(30.0, 0.0), (0.0, 30.0)] {
        let view = Tilted::new(
            chart,
            Tilt::new(&spec(t1, t2, TiltMap::Lean, x, x), centre, [SPAN, SPAN]),
        )
        .unwrap();
        // (a, b, d) from the closed form of R for one angle.
        let tilt = |u: f64, v: f64| {
            let (du, dv) = (u - centre[0], v - centre[1]);
            if t1 != 0.0 {
                (centre[0] + c * du, centre[1] + dv, SPAN * (s * du))
            } else {
                (centre[0] + du, centre[1] + c * dv, SPAN * (s * dv))
            }
        };
        for [u, v] in grid(24, 0.3, 1.45) {
            let (a, b, d) = tilt(u, v);
            if (0.0..=1.0).contains(&a) && (0.0..=1.0).contains(&b) {
                continue;
            }
            past += 1;
            let (ea, eb, ed) = if before {
                let (iu, iv) = sphere_image(u, v);
                tilt(iu, iv)
            } else {
                (a, b, d)
            };
            let moved = chart.sliced(x, held(&chart, x) + ed).unwrap();
            assert_eq!(
                bits(&view.decode::<Defaults, f64>(u, v)),
                bits(&moved.decode::<Defaults, f64>(ea, eb)),
                "the pixel ({u}, {v}) past the square is not the chart at the tilted (a, b)"
            );
            assert_eq!(view.validate(u, v), ValidationResult::Valid);
            let (ia, ib) = sphere_image(a, b);
            let (p, q) = (chart.point::<f64>(a, b), chart.point::<f64>(ia, ib));
            // The two angles differ by rounding alone: eight ulps of the angle's range, 2π.
            let tol = 8.0 * EPS * std::f64::consts::TAU;
            assert!(
                (0..3).all(|i| (p[i] - q[i]).abs() <= tol),
                "the point at the tilted ({a}, {b}) is not its image ({ia}, {ib}) inside the square: {p:?} vs {q:?}"
            );
        }
    }
    assert!(past > 100, "too few pixels past the square: {past}");
}

/// Both invariant charts tilted into a mass logit: past `s`'s edges and below `t = 0` not a system, past `t = 1` the
/// continuing side's system, inside the square a system, each classified by the axes' types on `(a, b)`. `before`
/// applies the types to `(u, v)` before the tilt (a control); `table` reads a per-chart table bounded on every side
/// (a control).
fn check_invariant(before: bool, table: bool) {
    fn one<C: kernel::chart::tilt::Tiltable<Param = f64> + Copy>(
        chart: C,
        before: bool,
        table: bool,
    ) -> [u32; 3] {
        let x = 6u32;
        let centre = [0.5, 0.45];
        let mut counts = [0; 3];
        // (spec, (a, b) and d at the offset (ũ, ṽ)): the single angles by R's closed form, Tip as sheet with both
        // angles by Rodrigues' formula.
        let (s, c) = sc(30.0);
        let r = rodrigues(35.0, -25.0);
        type Place = Box<dyn Fn(f64, f64) -> (f64, f64, f64)>;
        let cases: [(TiltSpec, Place); 3] = [
            (
                spec(30.0, 0.0, TiltMap::Sheet, x, x),
                Box::new(move |du, dv| (c * du, dv, SPAN * (s * du))),
            ),
            (
                spec(0.0, 30.0, TiltMap::Lean, x, x),
                Box::new(move |du, dv| (du, c * dv, SPAN * (s * dv))),
            ),
            (
                spec(35.0, -25.0, TiltMap::Sheet, x, x),
                Box::new(move |du, dv| {
                    (
                        r[0][0] * du + r[1][0] * dv,
                        r[0][1] * du + r[1][1] * dv,
                        SPAN * (r[0][2] * du + r[1][2] * dv),
                    )
                }),
            ),
        ];
        let inside = |w: f64| (0.0..=1.0).contains(&w);
        // Whether a coordinate lies within a hair of an edge, where rounding may put it either side.
        let near = |w: f64| w.abs() < 1e-9 || (w - 1.0).abs() < 1e-9;
        for (sp, place) in cases {
            let view = Tilted::new(chart, Tilt::new(&sp, centre, [SPAN, SPAN])).unwrap();
            for [u, v] in grid(29, -0.45, 1.45) {
                let (da, db, d) = place(u - centre[0], v - centre[1]);
                let (a, b) = (centre[0] + da, centre[1] + db);
                if near(a) || near(b) {
                    continue;
                }
                let (pa, pb) = if before { (u, v) } else { (a, b) };
                let bounded = !inside(pa) || pb < 0.0;
                let continuing = !bounded && pb > 1.0;
                let system = view.validate(u, v) == ValidationResult::Valid;
                if bounded || (continuing && table) {
                    counts[0] += 1;
                    assert!(
                        !system,
                        "{} under {sp:?}: the pixel ({u}, {v}) at (a, b) = ({a}, {b}) past a bounded edge is a system",
                        chart.name()
                    );
                } else {
                    counts[usize::from(!continuing) + 1] += 1;
                    assert!(
                        system,
                        "{} under {sp:?}: the pixel ({u}, {v}) at (a, b) = ({a}, {b}) is not a system",
                        chart.name()
                    );
                    if !before && !table {
                        let moved = chart.sliced(x, held(&chart, x) + d).unwrap();
                        let got = view.decode::<Defaults, f64>(u, v);
                        let want = moved.decode::<Defaults, f64>(a, b);
                        let close = (0..3).all(|i| {
                            (0..2).all(|k| {
                                (got.ic.r[i][k] - want.ic.r[i][k]).abs()
                                    <= 64.0 * EPS * (1.0 + want.ic.r[i][k].abs())
                                    && (got.ic.p[i][k] - want.ic.p[i][k]).abs()
                                        <= 64.0 * EPS * (1.0 + want.ic.p[i][k].abs())
                            })
                        });
                        assert!(
                            close && got.degenerate.is_none(),
                            "{} under {sp:?}: the pixel ({u}, {v}) is not the chart's system at (a, b) = ({a}, {b})",
                            chart.name()
                        );
                    }
                }
            }
        }
        counts
    }
    for counts in [
        one(InvariantLE(invariant()), before, table),
        one(InvariantLK(invariant()), before, table),
    ] {
        assert!(
            counts.iter().all(|&n| n > 20),
            "every case met (bounded, past t = 1, inside): {counts:?}"
        );
    }
}

/// A latent view saved as an oblique chart continues affinely past the square, `z₀ + (2s−1)q₁ + (2t−1)q₂`, to the
/// rounding of a three-term sum, and every point is a system. `clamped` expects the formula to stop at the square's
/// edge (the control).
fn check_oblique(clamped: bool) {
    let (q1, q2) =
        latent_basis(&frame(0.5), &spec(30.0, 50.0, TiltMap::Lean, 4, 5)).expect("hidden targets");
    let z0 = [0.3, -0.5, 0.2, -0.1, 0.15, 0.05, 0.7, -0.4];
    let chart = Latent { z0, q1, q2 };
    for [u, v] in [[1.4, -0.3], [-0.6, 1.9], [2.5, 2.5], [-3.0, 0.5]] {
        let w = |t: f64| if clamped { t.clamp(0.0, 1.0) } else { t };
        let (a, b) = (2.0 * w(u) - 1.0, 2.0 * w(v) - 1.0);
        let got = chart.map::<f64>(u, v).z.z;
        for i in 0..8 {
            let want = z0[i] + a * q1[i] + b * q2[i];
            let bound = 4.0 * EPS * (z0[i].abs() + (a * q1[i]).abs() + (b * q2[i]).abs());
            assert!(
                (got[i] - want).abs() <= bound,
                "the oblique chart at ({u}, {v}) does not continue affinely: z[{i}] = {} against {want}",
                got[i]
            );
        }
        assert_eq!(chart.validate(u, v), ValidationResult::Valid);
    }
}

#[test]
fn qa_tilted_extension() {
    check_pole(false);
    check_invariant(false, false);
    check_oblique(false);
}

negative_control!(
    qa_tilted_extension,
    "the pole crossing applied to (u, v) before the tilt is not the types acting on (a, b), so it must fail",
    expected = "is not the chart at the tilted (a, b)",
    check_pole(true)
);

mod invariant_before {
    use super::*;
    negative_control!(
        qa_tilted_extension,
        "the invariant chart's bounded edges read on (u, v) before the tilt misclassify pixels, so it must fail",
        expected = "InvariantLE under",
        check_invariant(true, false)
    );
}

mod invariant_table {
    use super::*;
    negative_control!(
        qa_tilted_extension,
        "a per-chart table bounded on every side drops the continuing t > 1 side, so it must fail",
        expected = "past a bounded edge is a system",
        check_invariant(false, true)
    );
}

mod oblique_clamped {
    use super::*;
    negative_control!(
        qa_tilted_extension,
        "an oblique chart stopped at the square's edge does not continue affinely, so it must fail",
        expected = "does not continue affinely",
        check_oblique(true)
    );
}
