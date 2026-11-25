//! A statistic's measure (design `principia_chart_composition.md` § 6.4; `engine::statistic`). Every test is named
//! `statistic_measure_…`, so `cargo test -p engine statistic_measure` runs them all, and each has its control.
//!
//! **The criteria, written before the first run.**
//! - *A known fraction through a power warp.* The chart's horizontal axis is `x = t³` on `[0, 1]`, its vertical
//!   `y = t` on `[0, 1]`, the set `x < 0.3, y < 0.625`, whose share in `(x, y)`'s own measure is `0.3·0.625 = 0.1875`.
//!   On a 2048 × 64 grid `y`'s edge falls on a pixel edge (`0.625 = 40/64`), so the weighted share's error is `x`'s:
//!   the one column the edge `t = 0.3^(1/3) ≈ 0.669` crosses, weight `3t²/2048 ≈ 6.6·10⁻⁴` of a total of 1, times
//!   `0.625`, and the midpoint rule's `O(1/2048²)`: **`|share − 0.1875| ≤ 10⁻³`**. The per-pixel count, `≈ 0.669·0.625`,
//!   and the inverted weight's share sit far outside it, which the test asserts too.
//! - *A paper's own measure.* The paper's coordinates are `x ∈ [0.5, 2.5]`, `y ∈ [1, 4]`, the chart's axes a power warp
//!   `γ = 2` on `x` and a log warp on `y`, the set the disc `x² + (y − 2)² < 1.5`. The flat 1024 × 1024 grid in `(x, y)`
//!   and the chart's 1024 × 1024 grid weighted into `(x, y)` are two midpoint sums of one area; each misses by at most the
//!   pixels its boundary crosses, about `2πr/(1024·area of the window) ≈ 1.3·10⁻³`, and less as their signs cancel:
//!   **`|chart − paper| ≤ 2·10⁻³`**.

use engine::statistic::WarpedAxis;
use engine::statistic::{
    fraction, fraction_guarded, warp_area, FlatGrid, Fraction, Measure, Refused, Sample, Weighting,
};
use kernel::chart::warp::AxisWarp;
use validation::negative_control;

/// `grid`'s samples, each pixel counted, inside where `inside` says at the chart point.
fn samples(grid: FlatGrid, inside: impl Fn(f64, f64) -> bool) -> Vec<Sample> {
    grid.centres()
        .map(|(u, v)| Sample {
            u,
            v,
            counted: true,
            inside: inside(u, v),
        })
        .collect()
}

/// The power fixture's chart (the module's note): `x = t³`, `y = t`, both on `[0, 1]`.
fn cube() -> (WarpedAxis, WarpedAxis) {
    (
        (AxisWarp::power(3.0).expect("γ > 0"), [0.0, 1.0]),
        (AxisWarp::linear(), [0.0, 1.0]),
    )
}

/// The power fixture's weighted share, its area factor `area`.
fn power_share(area: &dyn Fn(f64, f64) -> Option<f64>) -> (Fraction, Fraction) {
    let grid = FlatGrid {
        width: 2048,
        height: 64,
    };
    let (h, v) = cube();
    let s = samples(grid, |u, t| {
        h.0.value(h.1, u) < 0.3 && v.0.value(v.1, t) < 0.625
    });
    let weighted = fraction(
        grid,
        &s,
        Weighting::Named {
            name: "x·y, the axes' own",
            area,
        },
    )
    .expect("a flat grid, a named measure");
    let counted = fraction(grid, &s, Weighting::PerPixel).expect("a flat grid");
    (weighted, counted)
}

fn check_power(area: &dyn Fn(f64, f64) -> Option<f64>) {
    let (weighted, counted) = power_share(area);
    let share = weighted.share.expect("a defined share");
    println!(
        "the power fixture: weighted {share:.6} (want 0.1875), per pixel {:?}",
        counted.share
    );
    assert!(
        (share - 0.1875).abs() <= 1e-3,
        "the known fraction is not recovered through the power warp: {share}"
    );
    assert_eq!(weighted.measure.to_string(), "measure: x·y, the axes' own");
    assert_eq!(counted.measure, Measure::PerPixel);
    assert_eq!(counted.measure.to_string(), "per pixel of this chart");
    let per_pixel = counted.share.expect("a count");
    assert!(
        (per_pixel - 0.1875).abs() > 0.1,
        "the fixture cannot tell a weight from a count: {per_pixel}"
    );
    let (h, v) = cube();
    let inverted = move |u: f64, t: f64| Some(1.0 / (h.0.rate(h.1, u) * v.0.rate(v.1, t)).abs());
    let wrong = power_share(&inverted).0.share.expect("a defined share");
    assert!(
        (wrong - 0.1875).abs() > 0.1,
        "the fixture cannot tell the weight from its inverse: {wrong}"
    );
}

#[test]
fn statistic_measure_power_fixture() {
    let (h, v) = cube();
    check_power(&warp_area(h, v));
}

negative_control!(
    statistic_measure_power_fixture,
    "the weight inverted, 1/|x′y′|",
    expected = "not recovered through the power warp",
    {
        let (h, v) = cube();
        check_power(&move |u, t| Some(1.0 / (h.0.rate(h.1, u) * v.0.rate(v.1, t)).abs()))
    }
);

/// The paper's window and the chart's warps onto it (the module's note).
fn paper() -> (WarpedAxis, WarpedAxis) {
    (
        (AxisWarp::power(2.0).expect("γ > 0"), [0.5, 2.5]),
        (AxisWarp::log(), [1.0, 4.0]),
    )
}

fn disc(x: f64, y: f64) -> bool {
    x * x + (y - 2.0) * (y - 2.0) < 1.5
}

/// A quadtree's leaf centres over the unit square, split to `depth` where a leaf's centre is within its own size of
/// the disc's edge in the chart: the scheduler's sampling, never a statistic's.
#[cfg(feature = "controls")]
fn leaves(depth: u32) -> Vec<Sample> {
    let (h, v) = paper();
    let mut out = Vec::new();
    let mut stack = vec![(0.0f64, 0.0f64, 1.0f64, 0u32)];
    while let Some((u0, v0, size, d)) = stack.pop() {
        let (u, t) = (u0 + size / 2.0, v0 + size / 2.0);
        let (x, y) = (h.0.value(h.1, u), v.0.value(v.1, t));
        let edge = (x * x + (y - 2.0) * (y - 2.0) - 1.5).abs() < 4.0 * size;
        if d < depth && edge {
            let s = size / 2.0;
            stack.extend([
                (u0, v0, s, d + 1),
                (u0 + s, v0, s, d + 1),
                (u0, v0 + s, s, d + 1),
                (u0 + s, v0 + s, s, d + 1),
            ]);
        } else {
            out.push(Sample {
                u,
                v: t,
                counted: true,
                inside: disc(x, y),
            });
        }
    }
    out
}

/// The disc's share in the paper's measure on the warped chart, from `chart_samples`, against the flat grid in the
/// paper's coordinates.
fn check_paper(chart_samples: Option<Vec<Sample>>) {
    let n = 1024;
    let grid = FlatGrid {
        width: n,
        height: n,
    };
    let (h, v) = paper();
    let flat = samples(grid, |u, t| disc(lerp(h.1, u), lerp(v.1, t)));
    let in_paper = fraction(grid, &flat, Weighting::PerPixel)
        .expect("a flat grid")
        .share
        .expect("a count");
    let chart = chart_samples
        .unwrap_or_else(|| samples(grid, |u, t| disc(h.0.value(h.1, u), v.0.value(v.1, t))));
    let area = warp_area(h, v);
    let on_chart = match fraction(
        grid,
        &chart,
        Weighting::Named {
            name: "dx·dy, the paper's",
            area: &area,
        },
    ) {
        Ok(f) => f.share.expect("a defined share"),
        Err(r) => panic!("the chart's statistic is {r}"),
    };
    println!("the disc: on the chart {on_chart:.6}, the paper's flat grid {in_paper:.6}");
    assert!(
        (on_chart - in_paper).abs() <= 2e-3,
        "the paper's measure on the chart is not the flat grid in its coordinates: {on_chart} against {in_paper}"
    );
}

fn lerp(r: [f64; 2], t: f64) -> f64 {
    r[0] + (r[1] - r[0]) * t
}

#[test]
fn statistic_measure_paper_measure() {
    check_paper(None);
}

negative_control!(
    statistic_measure_paper_measure,
    "a count of quadtree leaves fed in",
    expected = "not a flat grid",
    check_paper(Some(leaves(10)))
);

/// The default and the cases with no area statistic: a count is labelled `per pixel of this chart`; a weight with a
/// measure named is that measure; a weight naming none is refused; a mid-morph frame admits none; a ray-degenerate
/// pixel makes the share "—", never 0%; pixels out of the count leave the total. `guarded` is the guard that a weight
/// names its measure, off for the control.
fn check_labels(guarded: bool) {
    let grid = FlatGrid {
        width: 8,
        height: 4,
    };
    let s = samples(grid, |u, _| u < 0.5);
    let count = fraction(grid, &s, Weighting::PerPixel).expect("a flat grid");
    assert_eq!(count.to_string(), "50.0000% (per pixel of this chart)");
    let one = |_: f64, _: f64| Some(1.0);
    match fraction_guarded(
        grid,
        &s,
        Weighting::Named {
            name: " ",
            area: &one,
        },
        guarded,
    ) {
        Err(Refused::NoMeasure) => {}
        r => panic!("a weight with no weight and no label named is not refused: {r:?}"),
    }
    let named = fraction(
        grid,
        &s,
        Weighting::Named {
            name: "chart area",
            area: &one,
        },
    )
    .expect("a named measure");
    assert_eq!(named.to_string(), "50.0000% (measure: chart area)");
    assert_eq!(
        fraction(grid, &s, Weighting::MidMorph),
        Err(Refused::MidMorph)
    );
    let degenerate = |u: f64, _: f64| (u > 0.1).then_some(1.0);
    let dash = fraction(
        grid,
        &s,
        Weighting::Named {
            name: "chart area",
            area: &degenerate,
        },
    )
    .expect("a named measure");
    assert_eq!(dash.share, None);
    assert_eq!(dash.to_string(), "— (measure: chart area)");
    let mut left = s.clone();
    for x in left.iter_mut().filter(|x| x.u > 0.75) {
        x.counted = false;
    }
    let share = fraction(grid, &left, Weighting::PerPixel)
        .expect("a flat grid")
        .share
        .expect("a count");
    assert_eq!(
        share,
        4.0 / 6.0,
        "pixels out of the count stay in the total"
    );
}

#[test]
fn statistic_measure_labels() {
    check_labels(true);
}

negative_control!(
    statistic_measure_labels,
    "no weight and no label: the guard that a weight names its measure off",
    expected = "no weight and no label named is not refused",
    check_labels(false)
);
