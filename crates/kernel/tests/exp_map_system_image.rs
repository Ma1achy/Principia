//! The shape sphere's `system_image` and label, and the exponential map's cap and per-pixel fold
//! (`kernel::chart::shape_sphere`; chart_reference §3.3; R-141, R-503; TASK-M2-28).
//! - `exp_map_system_image_one_hemisphere`: caps inside `w ≥ 0`, inside `w ≤ 0`, and touching the equator from one
//!   side, from above and from below, report bijective, every pixel of the cap counted; a centre below the equator under
//!   corners above it, and corners just past the equator, straddle (REQ-CHART-075).
//! - `exp_map_system_image_fold_pairs`: a straddling cap on a grid symmetric under `w → −w` (`n₀` on the equator,
//!   `e₂ = ŵ`, odd `N`) reports the per-pixel fold; each `w < 0` pixel folds onto its mirror `(i, N − 1 − j)`, which
//!   counts, and the counted total is the hand count, `N(N + 1)/2`, where `s√2 < π` (REQ-CHART-075).
//! - `exp_map_system_image_equator_row`: on that grid the middle row's computed `w` is ±0.0 and each of its pixels
//!   counts once (REQ-CHART-075).
//! - `exp_map_system_image_domain_end`: on the same grid with `s√2·(N − 1)/N > π`, every pixel centre more than `10⁻⁶`
//!   from `r = π`, the total is the number of pixels with `j ≥ (N − 1)/2` and `r ≤ π`, enumerated from `r` alone, and
//!   each pixel past `r = π` is the domain's end (REQ-CHART-075).
//! - `exp_map_system_image_antipode`: pixels exactly on `r = π` are one system, `−n₀`, and count once together, by
//!   the per-pixel rule applied to `−n₀` (REQ-CHART-078).
//! - `exp_map_system_image_in_view`: the mirror test's log map, and its singular points: `n₀` always in view, `−n₀`
//!   only where the circle `r = π` meets `[0,1]²`, its edge case `s√2 = π` included (REQ-CHART-078).
//! - `shape_sphere_label`: the spherical map's label flags the redundancy, n-to-1 with n = 2 (R-141); a bijective cap's
//!   names its one hemisphere; a straddling cap's names the fold (REQ-CHART-002).

use kernel::chart::shape_sphere::{pixel_centre, Cap, PixelCount, ShapeMap, ShapeSphere};
use kernel::chart::SystemImage;
use validation::negative_control;

const PI: f64 = std::f64::consts::PI;

/// An exponential-map chart about the orthonormal frame `(n0, e1, e2)` with extent `scale`; the held blocks are
/// arbitrary, since the cap reads only the frame and the extent.
fn exp_chart(n0: [f64; 3], e1: [f64; 3], e2: [f64; 3], scale: f64) -> ShapeSphere<f64> {
    ShapeSphere {
        map: ShapeMap::Exponential,
        n0,
        e1,
        e2,
        scale,
        inertia: 1.0,
        phase: 0.0,
        z_mass: [0.0, 0.0],
        z_mom: [0.0; 4],
    }
}

/// A cap about the point at polar angle `alpha` from `+w` in the `(u, w)` plane, with extent `scale`, its frame turned
/// by 45° so that the corner `(0, 0)` lies down the meridian, at polar angle `alpha + s√2`, and `(1, 1)` up it:
/// `e₁ = (m − ŷ)/√2`, `e₂ = (m + ŷ)/√2`, `m` the meridian's direction toward `+w`.
fn tilted(alpha: f64, scale: f64) -> ShapeSphere<f64> {
    let k = std::f64::consts::FRAC_1_SQRT_2;
    let m = [-alpha.cos(), 0.0, alpha.sin()];
    exp_chart(
        [alpha.sin(), 0.0, alpha.cos()],
        [k * m[0], -k, k * m[2]],
        [k * m[0], k, k * m[2]],
        scale,
    )
}

/// A cap about a pole, `n₀ = ±ŵ`, `e₁ = x̂`, `e₂ = ŷ`, with extent `scale`.
fn polar_cap(sign: f64, scale: f64) -> ShapeSphere<f64> {
    exp_chart([0.0, 0.0, sign], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], scale)
}

/// The symmetric grid's chart (R-503, review rounds 1 and 2): `n₀ = x̂` on the equator, `e₁ = ŷ`, `e₂ = ŵ`, so the
/// mirror `w → −w` is `(u, v) ↔ (u, 1 − v)`.
fn symmetric(scale: f64) -> ShapeSphere<f64> {
    exp_chart([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], scale)
}

/// The odd resolution of the symmetric grid.
const N: u32 = 9;

/// The middle row, `(N − 1)/2`, at `v = ½`.
const MID: u32 = (N - 1) / 2;

/// The symmetric grid's extent with every corner short of `r = π`, `s√2 < π`.
const S_INSIDE: f64 = 1.0;

/// The symmetric grid's extent with `s√2·(N − 1)/N > π`: a pixel centre of the counted half past `r = π`.
const S_PAST: f64 = 2.6;

/// A classification of the pixel at `(u, v)`: the kernel's [`ShapeSphere::exp_pixel`], or a control's altered one.
type Classify = fn(&ShapeSphere<f64>, f64, f64) -> PixelCount;

/// The kernel's per-pixel rule.
fn kernel_rule(chart: &ShapeSphere<f64>, u: f64, v: f64) -> PixelCount {
    chart.exp_pixel(u, v)
}

/// The total `classify` counts over the `N` × `N` grid of pixel centres: one per counted pixel. No pixel lies on
/// `r = π` here.
fn total_by(chart: &ShapeSphere<f64>, classify: Classify) -> u64 {
    let mut total = 0;
    for j in 0..N {
        for i in 0..N {
            let c = classify(chart, pixel_centre(i, N), pixel_centre(j, N));
            total += u64::from(c == PixelCount::Counted);
        }
    }
    total
}

/// The symmetric grid's pixel `(i, j)`'s `r`, from `(u, v)` alone: `s·√((2u − 1)² + (2v − 1)²)`.
fn grid_r(scale: f64, i: u32, j: u32) -> f64 {
    let a = 2.0 * pixel_centre(i, N) - 1.0;
    let b = 2.0 * pixel_centre(j, N) - 1.0;
    scale * (a * a + b * b).sqrt()
}

// ── One hemisphere ─────────────────────────────────────────────────────────────────────────────────────────────────

/// The caps: inside `w ≥ 0` and inside `w ≤ 0`, tilted off the poles, the caps about each pole whose corners reach
/// the equator, `s√2 = π/2`, their computed corner `w` `cos(π/2)`'s `6·10⁻¹⁷` of the cap's own sign, and the cap on
/// the equator alone, `s = 0`, which reads `w ≥ 0`.
fn one_hemisphere_caps() -> Vec<(&'static str, ShapeSphere<f64>, Cap)> {
    let touch = PI / 2.0 / 2f64.sqrt();
    let (up, down) = (polar_cap(1.0, touch), polar_cap(-1.0, touch));
    for (chart, sign) in [(&up, 1.0), (&down, -1.0)] {
        let w = chart.exp_at(1.0, 1.0).0[2];
        assert!(
            sign * w > 0.0 && w.abs() < 1e-15,
            "the touching cap's corner is at w = {w:e}, not on the equator from its own side"
        );
    }
    vec![
        ("the cap in w ≥ 0", tilted(0.6, 0.6), Cap::Upper),
        ("the cap in w ≤ 0", tilted(PI - 0.6, 0.6), Cap::Lower),
        ("the cap touching the equator from above", up, Cap::Upper),
        ("the cap touching the equator from below", down, Cap::Lower),
        ("the cap on the equator alone", symmetric(0.0), Cap::Upper),
    ]
}

/// Each cap lies within the closed hemisphere named, reports bijective through `image`, and counts each pixel of a
/// 16 × 16 grid once.
fn check_one_hemisphere(image: fn(&ShapeSphere<f64>) -> SystemImage) {
    for (name, chart, cap) in one_hemisphere_caps() {
        assert_eq!(chart.cap(), cap, "{name}: the cap's hemisphere");
        assert_eq!(
            image(&chart),
            SystemImage::Bijective,
            "{name}: a cap within one closed hemisphere is not bijective"
        );
        assert_eq!(
            chart.exp_counted_total(16, 16),
            256,
            "{name}: every pixel of the cap counts once"
        );
    }
}

#[test]
fn exp_map_system_image_one_hemisphere() {
    check_one_hemisphere(ShapeSphere::system_image);
    // A corner a nanoradian short of the equator, then a nanoradian past it: the decision is the corners'.
    let edge = (PI / 2.0 - 0.6) / 2f64.sqrt();
    assert_eq!(tilted(0.6, edge - 1e-9).cap(), Cap::Upper);
    let past = tilted(0.6, edge + 1e-9);
    assert_eq!(past.cap(), Cap::Straddling);
    assert_eq!(past.system_image(), SystemImage::NToOne(2));
    // About the south pole with π/2 < s√2 < π every corner lies above the equator and the centre below it: the centre
    // decides that the cap straddles.
    let ring = polar_cap(-1.0, 2.0);
    for [u, v] in [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]] {
        assert!(
            ring.exp_at(u, v).0[2] > 0.0,
            "the ring's corner ({u}, {v}) is not above the equator"
        );
    }
    assert_eq!(ring.cap(), Cap::Straddling);
    // A corner on r = π: the cap reaches −n₀ and straddles.
    assert_eq!(polar_cap(1.0, PI / 2f64.sqrt()).cap(), Cap::Straddling);
}

negative_control!(
    exp_map_system_image_one_hemisphere,
    "R-141's fixed n = 2 read for the exponential map reports a cap in one hemisphere 2-to-1, so it must fail",
    expected = "is not bijective",
    {
        check_one_hemisphere(|_| SystemImage::NToOne(2));
    }
);

// ── The straddling cap's fold ──────────────────────────────────────────────────────────────────────────────────────

/// On the symmetric grid at `s√2 < π`: the per-pixel fold, each `w < 0` pixel folded onto its mirror, which counts,
/// and the total `classify` gives equal to the hand count `N(N + 1)/2`, the rows `j ≥ (N − 1)/2`.
fn check_fold_pairs(classify: Classify) {
    let chart = symmetric(S_INSIDE);
    assert!(S_INSIDE * 2f64.sqrt() < PI);
    assert_eq!(chart.cap(), Cap::Straddling);
    assert_eq!(chart.system_image(), SystemImage::NToOne(2));
    for j in 0..MID {
        for i in 0..N {
            let (u, v, mirror) = (
                pixel_centre(i, N),
                pixel_centre(j, N),
                pixel_centre(N - 1 - j, N),
            );
            assert!(
                chart.exp_at(u, v).0[2] < 0.0,
                "({i}, {j}) is not below the equator"
            );
            assert_eq!(chart.exp_pixel(u, v), PixelCount::Folded, "({i}, {j})");
            assert_eq!(
                chart.exp_pixel(u, mirror),
                PixelCount::Counted,
                "({i}, {})",
                N - 1 - j
            );
        }
    }
    let hand = u64::from(N * (N + 1) / 2);
    assert_eq!(
        total_by(&chart, classify),
        hand,
        "the counted total is not the hand count by the per-pixel rule"
    );
    assert_eq!(chart.exp_counted_total(N, N), hand);
}

#[test]
fn exp_map_system_image_fold_pairs() {
    check_fold_pairs(kernel_rule);
}

negative_control!(
    exp_map_system_image_fold_pairs,
    "a fold that counts a mirrored pair twice counts the w < 0 half too, so it must fail",
    expected = "is not the hand count",
    {
        check_fold_pairs(|chart, u, v| match chart.exp_pixel(u, v) {
            PixelCount::Folded => PixelCount::Counted,
            c => c,
        });
    }
);

/// On the symmetric grid the middle row's computed `w` is ±0.0, each of its pixels its own mirror, and `classify`
/// counts each once.
fn check_equator_row(classify: Classify) {
    let chart = symmetric(S_INSIDE);
    for i in 0..N {
        let (u, v) = (pixel_centre(i, N), pixel_centre(MID, N));
        assert_eq!(chart.exp_at(u, v).0[2], 0.0, "({i}, {MID})'s w is not ±0.0");
        assert_eq!(
            classify(&chart, u, v),
            PixelCount::Counted,
            "the equator pixel ({i}, {MID}) does not count once"
        );
    }
    assert_eq!(total_by(&chart, classify), u64::from(N * (N + 1) / 2));
}

#[test]
fn exp_map_system_image_equator_row() {
    check_equator_row(kernel_rule);
}

negative_control!(
    exp_map_system_image_equator_row,
    "a fold that drops the w = 0 row reads w > 0 for the representative, so it must fail",
    expected = "does not count once",
    {
        check_equator_row(|chart, u, v| {
            let w = chart.exp_at(u, v).0[2];
            match chart.exp_pixel(u, v) {
                PixelCount::Counted if w == 0.0 => PixelCount::Folded,
                c => c,
            }
        });
    }
);

/// On the symmetric grid past `r = π`: every pixel centre more than `10⁻⁶` from it, one of the counted half past it,
/// each pixel past it the domain's end, and the total `classify` gives equal to the number of pixels with
/// `j ≥ (N − 1)/2` and `r ≤ π`, enumerated from `r` alone.
fn check_domain_end(classify: Classify) {
    let chart = symmetric(S_PAST);
    assert!(S_PAST * 2f64.sqrt() * f64::from(N - 1) / f64::from(N) > PI);
    let mut hand = 0;
    let mut past = 0;
    for j in 0..N {
        for i in 0..N {
            let r = grid_r(S_PAST, i, j);
            assert!(
                (r - PI).abs() > 1e-6,
                "({i}, {j}) lies within 1e-6 of r = π"
            );
            if r > PI {
                past += u64::from(j >= MID);
                let c = chart.exp_pixel(pixel_centre(i, N), pixel_centre(j, N));
                assert_eq!(c, PixelCount::DomainEnd, "({i}, {j}) past r = π");
            } else {
                hand += u64::from(j >= MID);
            }
        }
    }
    assert!(past > 0, "no pixel of the counted half lies past r = π");
    assert_eq!(
        total_by(&chart, classify),
        hand,
        "the total is not the count of the pixels with j ≥ (N − 1)/2 and r ≤ π"
    );
    assert_eq!(chart.exp_counted_total(N, N), hand);
}

#[test]
fn exp_map_system_image_domain_end() {
    check_domain_end(kernel_rule);
}

negative_control!(
    exp_map_system_image_domain_end,
    "a count that includes the pixels past r = π counts the domain's end, so it must fail",
    expected = "is not the count of the pixels",
    {
        check_domain_end(|chart, u, v| match chart.exp_pixel(u, v) {
            PixelCount::DomainEnd => PixelCount::Counted,
            c => c,
        });
    }
);

// ── r = π (REQ-CHART-078) ──────────────────────────────────────────────────────────────────────────────────────────

/// A 3 × 2 grid at `s = 2π` about `n₀` with `e₂ = ŷ` and `e₁` the third direction: the middle column's two pixel
/// centres, `(2u − 1) = 0` and `(2v − 1) = ∓½`, so `d = ∓π·ŷ`, lie exactly on `r = π`, the side columns past it.
fn antipode_grid(n0: [f64; 3], e1: [f64; 3]) -> ShapeSphere<f64> {
    exp_chart(n0, e1, [0.0, 1.0, 0.0], 2.0 * PI)
}

/// The antipode grids' totals by `total`: `−n₀` counted once where it counts by the per-pixel rule (`n₀ = −ŵ`, its
/// `w = 1`; `n₀ = x̂`, its `w = −0.0`), and not at all where its mirror, `n₀`, is in view (`n₀ = ŵ`).
fn check_antipode(total: fn(&ShapeSphere<f64>, u32, u32) -> u64) {
    for (n0, e1, counts) in [
        ([0.0, 0.0, -1.0], [1.0, 0.0, 0.0], 1),
        ([1.0, 0.0, 0.0], [0.0, 0.0, 1.0], 1),
        ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], 0),
    ] {
        let chart = antipode_grid(n0, e1);
        for j in 0..2 {
            let (u, v) = (pixel_centre(1, 3), pixel_centre(j, 2));
            assert_eq!(chart.exp_at(u, v).1, PI, "({u}, {v}) is not on r = π");
            assert_eq!(chart.exp_pixel(u, v), PixelCount::Antipode);
            assert_eq!(
                chart.exp_pixel(pixel_centre(0, 3), v),
                PixelCount::DomainEnd
            );
        }
        assert_eq!(chart.antipode_counts(), counts == 1, "n₀ = {n0:?}");
        assert_eq!(
            total(&chart, 3, 2),
            counts,
            "n₀ = {n0:?}: the pixels on r = π are not counted once together"
        );
    }
}

#[test]
fn exp_map_system_image_antipode() {
    check_antipode(ShapeSphere::exp_counted_total);
}

negative_control!(
    exp_map_system_image_antipode,
    "counting each pixel on r = π as a system of its own counts −n₀ twice, so it must fail",
    expected = "are not counted once together",
    {
        check_antipode(|chart, nx, ny| {
            let mut total = 0;
            for j in 0..ny {
                for i in 0..nx {
                    let c = chart.exp_pixel(pixel_centre(i, nx), pixel_centre(j, ny));
                    total += u64::from(matches!(c, PixelCount::Counted | PixelCount::Antipode));
                }
            }
            total
        });
    }
);

/// The mirror test through `in_view`: a point whose log map lands inside `[0,1]²` is in view and one outside is not;
/// `n₀` is in view; `−n₀` is in view exactly where a corner reaches `r = π`, at `s√2 = π` (the computed corner `r`
/// exactly π) and past it, and not short of it.
fn check_in_view(in_view: fn(&ShapeSphere<f64>, [f64; 3]) -> bool) {
    let edge = PI / 2f64.sqrt();
    let short = polar_cap(1.0, edge.next_down());
    let at = polar_cap(1.0, edge);
    assert_eq!(
        at.exp_at(1.0, 1.0).1,
        PI,
        "the corner at s = π/√2 is not on r = π"
    );
    assert!(short.exp_at(1.0, 1.0).1 < PI);
    for chart in [&short, &at] {
        assert!(in_view(chart, [0.0, 0.0, 1.0]), "n₀ is not in view");
        let inside = chart.exp_at(0.3, 0.8).0;
        assert!(in_view(chart, inside), "a point of the cap is not in view");
    }
    assert!(
        !in_view(&short, [0.0, 0.0, -1.0]),
        "−n₀ is in view short of r = π"
    );
    assert!(
        in_view(&at, [0.0, 0.0, -1.0]),
        "−n₀ is not in view at r = π"
    );
    // Off the square: the point at d = (1.2·s, 0), a cap of extent s = 0.5.
    let small = polar_cap(1.0, 0.5);
    let off = [0.6f64.sin(), 0.0, 0.6f64.cos()];
    assert!(!in_view(&small, off), "a point off the square is in view");
    let on = [0.4f64.sin(), 0.0, 0.4f64.cos()];
    assert!(in_view(&small, on), "a point on the square is not in view");
}

#[test]
fn exp_map_system_image_in_view() {
    check_in_view(ShapeSphere::in_view);
}

negative_control!(
    exp_map_system_image_in_view,
    "the log map's centre taken for −n₀ puts the antipode in view on every cap, so it must fail",
    expected = "−n₀ is in view short of r = π",
    {
        check_in_view(|chart, m| {
            let antipode = m == [0.0, 0.0, -1.0];
            antipode || chart.in_view(m)
        });
    }
);

// ── The label (REQ-CHART-002) ──────────────────────────────────────────────────────────────────────────────────────

/// The label `label` gives each chart: the spherical map's n-to-1 with n = 2 and the redundancy flagged; a bijective
/// cap's one hemisphere; a straddling cap's per-pixel fold.
fn check_label(label: fn(&ShapeSphere<f64>) -> String) {
    let mut spherical = symmetric(1.0);
    spherical.map = ShapeMap::Spherical;
    assert_eq!(spherical.system_image(), SystemImage::NToOne(2));
    let cases = [
        (
            spherical,
            "ShapeSphere · system_image 2-to-1 · both φ hemispheres drawn, redundant: (θ, φ) and (θ, π − φ) decode to \
             one system, its w ≥ 0 point canonical",
        ),
        (
            tilted(0.6, 0.6),
            "ShapeSphere · system_image bijective · one hemisphere, w ≥ 0: each system in the cap once",
        ),
        (
            tilted(PI - 0.6, 0.6),
            "ShapeSphere · system_image bijective · one hemisphere, w ≤ 0: each system in the cap once",
        ),
        (
            symmetric(1.0),
            "ShapeSphere · system_image 2-to-1 · the cap straddles the equator, a per-pixel fold: a pixel whose mirror \
             w → −w is in view counts once with it, at its w > 0 pixel",
        ),
    ];
    for (chart, want) in cases {
        assert_eq!(label(&chart), want, "the label does not say its hemisphere");
    }
}

#[test]
fn shape_sphere_label() {
    check_label(|chart| chart.label().to_string());
}

negative_control!(
    shape_sphere_label,
    "a label that names no hemisphere, R-141's 2-to-1 on every cap, must fail",
    expected = "does not say its hemisphere",
    {
        check_label(|chart| {
            let mut spherical = *chart;
            spherical.map = ShapeMap::Spherical;
            spherical.label().to_string()
        });
    }
);
