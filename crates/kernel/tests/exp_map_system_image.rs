//! The shape sphere's `system_image` and label, and the exponential map's cap and count
//! (`kernel::chart::shape_sphere`; chart_reference §3.3; R-503, R-544; TASK-M2-28).
//! - `exp_map_system_image_one_hemisphere`: caps inside `w ≥ 0`, inside `w ≤ 0`, and touching the equator from one
//!   side, from above and from below, report bijective, every pixel of the cap counted once (REQ-CHART-075).
//! - `exp_map_system_image_straddling`: a straddling cap on a grid symmetric under `w → −w` (`n₀` on the equator,
//!   `e₂ = ŵ`, odd `N`, `s√2 < π`) reports bijective; every pixel of both halves is its own system, each `w < 0` pixel
//!   and its mirror `(i, N − 1 − j)` two, the middle row's computed `w` ±0.0 and each of its pixels counted once, and
//!   the counted total is `N²` (REQ-CHART-075).
//! - `exp_map_system_image_domain_end`: on the same grid with `s√2·(N − 1)/N > π`, every pixel centre more than `10⁻⁶`
//!   from `r = π`, the total is the number of pixels with `r ≤ π`, enumerated from `r` alone, and each pixel past
//!   `r = π` is the domain's end (REQ-CHART-075).
//! - `exp_map_system_image_antipode`: pixels exactly on `r = π` are one system, `−n₀`, and count once together, about
//!   a centre in either hemisphere or on the equator (REQ-CHART-078).
//! - `shape_sphere_label`: the label of either map names no hemisphere and flags no redundancy: the whole sphere, or
//!   the cap, bijective (REQ-CHART-002).
//!
//! Each test registers its negative control (R-176).

use kernel::chart::shape_sphere::{pixel_centre, PixelCount, ShapeMap, ShapeSphere};
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

/// The symmetric grid's extent with `s√2·(N − 1)/N > π`: a pixel centre past `r = π`.
const S_PAST: f64 = 2.6;

/// A counted total over the `N` × `N` grid of pixel centres: the kernel's [`ShapeSphere::exp_counted_total`], or a
/// control's altered one.
type Total = fn(&ShapeSphere<f64>) -> u64;

/// The kernel's counted total over the `N` × `N` grid.
fn kernel_total(chart: &ShapeSphere<f64>) -> u64 {
    chart.exp_counted_total(N, N)
}

/// The symmetric grid's pixel `(i, j)`'s `r`, from `(u, v)` alone: `s·√((2u − 1)² + (2v − 1)²)`.
fn grid_r(scale: f64, i: u32, j: u32) -> f64 {
    let a = 2.0 * pixel_centre(i, N) - 1.0;
    let b = 2.0 * pixel_centre(j, N) - 1.0;
    scale * (a * a + b * b).sqrt()
}

// ── One hemisphere ─────────────────────────────────────────────────────────────────────────────────────────────────

/// The caps, each with its hemisphere's sign: inside `w ≥ 0` and inside `w ≤ 0`, tilted off the poles, and the caps
/// about each pole whose corners reach the equator, `s√2 = π/2`, their computed corner `w` `cos(π/2)`'s `6·10⁻¹⁷` of
/// the cap's own sign.
fn one_hemisphere_caps() -> Vec<(&'static str, ShapeSphere<f64>, f64)> {
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
        ("the cap in w ≥ 0", tilted(0.6, 0.6), 1.0),
        ("the cap in w ≤ 0", tilted(PI - 0.6, 0.6), -1.0),
        ("the cap touching the equator from above", up, 1.0),
        ("the cap touching the equator from below", down, -1.0),
    ]
}

/// Each cap lies within its closed hemisphere at its centre and corners, reports bijective through `image`, and counts
/// each pixel of a 16 × 16 grid once.
fn check_one_hemisphere(image: fn(&ShapeSphere<f64>) -> SystemImage) {
    for (name, chart, sign) in one_hemisphere_caps() {
        for [u, v] in [[0.5, 0.5], [0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]] {
            let w = chart.exp_at(u, v).0[2];
            assert!(
                sign * w >= 0.0,
                "{name}: ({u}, {v}) at w = {w} leaves its hemisphere"
            );
        }
        assert_eq!(
            image(&chart),
            SystemImage::Bijective,
            "{name}: the cap is not bijective"
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
}

negative_control!(
    exp_map_system_image_one_hemisphere,
    "R-141's fixed n = 2 read for the exponential map reports a cap 2-to-1, so it must fail",
    expected = "is not bijective",
    {
        check_one_hemisphere(|_| SystemImage::NToOne(2));
    }
);

// ── The straddling cap: every pixel its own system ─────────────────────────────────────────────────────────────────

/// On the symmetric grid at `s√2 < π`: bijective; each `w < 0` pixel and its mirror `(i, N − 1 − j)`, `w > 0`, are
/// mirror points and two systems, each counted; the middle row's computed `w` is ±0.0, each of its pixels counted
/// once; and the total `total` gives is `N²`.
fn check_straddling(total: Total) {
    let chart = symmetric(S_INSIDE);
    assert!(S_INSIDE * 2f64.sqrt() < PI);
    assert_eq!(chart.system_image(), SystemImage::Bijective);
    for i in 0..N {
        for j in 0..MID {
            let (u, v, mirror) = (
                pixel_centre(i, N),
                pixel_centre(j, N),
                pixel_centre(N - 1 - j, N),
            );
            let (low, high) = (chart.exp_at(u, v).0, chart.exp_at(u, mirror).0);
            assert!(low[2] < 0.0, "({i}, {j}) is not below the equator");
            assert!(
                (0..2).all(|k| (low[k] - high[k]).abs() < 1e-15)
                    && (low[2] + high[2]).abs() < 1e-15,
                "({i}, {j}) and ({i}, {}) are not mirror points",
                N - 1 - j
            );
            assert_eq!(chart.exp_pixel(u, v), PixelCount::System, "({i}, {j})");
            assert_eq!(
                chart.exp_pixel(u, mirror),
                PixelCount::System,
                "({i}, {})",
                N - 1 - j
            );
        }
        let (u, v) = (pixel_centre(i, N), pixel_centre(MID, N));
        assert_eq!(chart.exp_at(u, v).0[2], 0.0, "({i}, {MID})'s w is not ±0.0");
        assert_eq!(chart.exp_pixel(u, v), PixelCount::System, "({i}, {MID})");
    }
    assert_eq!(
        total(&chart),
        u64::from(N * N),
        "the counted total is not N², every pixel of both halves once"
    );
}

#[test]
fn exp_map_system_image_straddling() {
    check_straddling(kernel_total);
}

negative_control!(
    exp_map_system_image_straddling,
    "R-503's per-pixel fold, a w < 0 pixel counting 0 when its mirror is in view, undercounts, so it must fail",
    expected = "is not N²",
    {
        check_straddling(|chart| {
            let mut total = 0;
            for j in 0..N {
                for i in 0..N {
                    let (u, v) = (pixel_centre(i, N), pixel_centre(j, N));
                    let mirror = chart.exp_pixel(u, pixel_centre(N - 1 - j, N));
                    let folded = chart.exp_at(u, v).0[2] < 0.0 && mirror == PixelCount::System;
                    total += u64::from(chart.exp_pixel(u, v) == PixelCount::System && !folded);
                }
            }
            total
        });
    }
);

// ── Past r = π ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// On the symmetric grid past `r = π`: every pixel centre more than `10⁻⁶` from it, one past it, each pixel past it the
/// domain's end and each short of it a system, and the total `total` gives equal to the number of pixels with
/// `r ≤ π`, enumerated from `r` alone.
fn check_domain_end(total: Total) {
    let chart = symmetric(S_PAST);
    assert!(S_PAST * 2f64.sqrt() * f64::from(N - 1) / f64::from(N) > PI);
    assert_eq!(chart.system_image(), SystemImage::Bijective);
    let mut hand = 0;
    let mut past = 0;
    for j in 0..N {
        for i in 0..N {
            let r = grid_r(S_PAST, i, j);
            assert!(
                (r - PI).abs() > 1e-6,
                "({i}, {j}) lies within 1e-6 of r = π"
            );
            let c = chart.exp_pixel(pixel_centre(i, N), pixel_centre(j, N));
            if r > PI {
                past += 1;
                assert_eq!(c, PixelCount::DomainEnd, "({i}, {j}) past r = π");
            } else {
                hand += 1;
                assert_eq!(c, PixelCount::System, "({i}, {j}) short of r = π");
            }
        }
    }
    assert!(past > 0, "no pixel lies past r = π");
    assert_eq!(
        total(&chart),
        hand,
        "the total is not the count of the pixels with r ≤ π"
    );
}

#[test]
fn exp_map_system_image_domain_end() {
    check_domain_end(kernel_total);
}

negative_control!(
    exp_map_system_image_domain_end,
    "a count that includes the pixels past r = π counts the domain's end, so it must fail",
    expected = "is not the count of the pixels",
    {
        check_domain_end(|chart| {
            let mut total = 0;
            for j in 0..N {
                for i in 0..N {
                    let c = chart.exp_pixel(pixel_centre(i, N), pixel_centre(j, N));
                    total += u64::from(c != PixelCount::Antipode);
                }
            }
            total
        });
    }
);

// ── r = π (REQ-CHART-078) ──────────────────────────────────────────────────────────────────────────────────────────

/// A 3 × 2 grid at `s = 2π` about `n₀` with `e₂ = ŷ` and `e₁` the third direction: the middle column's two pixel
/// centres, `(2u − 1) = 0` and `(2v − 1) = ∓½`, so `d = ∓π·ŷ`, lie exactly on `r = π`, the side columns past it.
fn antipode_grid(n0: [f64; 3], e1: [f64; 3]) -> ShapeSphere<f64> {
    exp_chart(n0, e1, [0.0, 1.0, 0.0], 2.0 * PI)
}

/// The antipode grids' totals by `total`: the two pixels on `r = π` reach `−n₀` and count once together, about a
/// centre below the equator, on it and above it alike; the side columns are the domain's end.
fn check_antipode(total: fn(&ShapeSphere<f64>, u32, u32) -> u64) {
    for (n0, e1) in [
        ([0.0, 0.0, -1.0], [1.0, 0.0, 0.0]),
        ([1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
        ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
    ] {
        let chart = antipode_grid(n0, e1);
        for j in 0..2 {
            let (u, v) = (pixel_centre(1, 3), pixel_centre(j, 2));
            let (n, r) = chart.exp_at(u, v);
            assert_eq!(r, PI, "({u}, {v}) is not on r = π");
            assert!(
                (0..3).all(|k| (n[k] + n0[k]).abs() < 1e-15),
                "({u}, {v}) reaches {n:?}, not −n₀"
            );
            assert_eq!(chart.exp_pixel(u, v), PixelCount::Antipode);
            for i in [0, 2] {
                assert_eq!(
                    chart.exp_pixel(pixel_centre(i, 3), v),
                    PixelCount::DomainEnd
                );
            }
        }
        assert_eq!(chart.system_image(), SystemImage::Bijective);
        assert_eq!(
            total(&chart, 3, 2),
            1,
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
                    total += u64::from(c != PixelCount::DomainEnd);
                }
            }
            total
        });
    }
);

// ── The label (REQ-CHART-002) ──────────────────────────────────────────────────────────────────────────────────────

/// The label `label` gives each chart: the spherical map's whole sphere, and the exponential map's cap, whether in one
/// hemisphere, straddling, or reaching `r = π`, each bijective, naming no hemisphere and flagging no redundancy.
fn check_label(label: fn(&ShapeSphere<f64>) -> String) {
    let mut spherical = symmetric(1.0);
    spherical.map = ShapeMap::Spherical;
    let cap = "ShapeSphere · system_image bijective · the cap r ≤ π about n₀: each pixel its own system, those on \
               r = π the one system −n₀, past r = π the domain's end";
    let cases = [
        (
            spherical,
            "ShapeSphere · system_image bijective · the whole sphere, pole to pole: each pixel its own system",
        ),
        (tilted(0.6, 0.6), cap),
        (tilted(PI - 0.6, 0.6), cap),
        (symmetric(1.0), cap),
        (symmetric(S_PAST), cap),
    ];
    for (chart, want) in cases {
        let got = label(&chart);
        assert!(
            !got.contains("hemisphere") && !got.contains("redundan"),
            "the label `{got}` names a hemisphere or flags a redundancy"
        );
        assert_eq!(got, want, "the label is not the chart's");
    }
}

#[test]
fn shape_sphere_label() {
    check_label(|chart| chart.label().to_string());
}

negative_control!(
    shape_sphere_label,
    "R-141's label, both φ hemispheres drawn and the redundancy flagged, must fail",
    expected = "names a hemisphere or flags a redundancy",
    {
        check_label(|_| {
            "ShapeSphere · system_image 2-to-1 · both φ hemispheres drawn, redundant: (θ, φ) and (θ, π − φ) decode to \
             one system, its w ≥ 0 point canonical"
                .to_owned()
        });
    }
);
