//! The shape-sphere scenes (`validation::shape_scene`; TASK-M2-28), rendered through the render harness and held to
//! their CPU twins and to the chart:
//! - `shape_scene_renders_its_twin_clear_of_ties`: every scene renders its CPU twin, far enough from every rounding tie
//!   that one reference holds on every backend, each sample's quadrant and hemisphere far from their sign's flip
//!   (REQ-CHART-002, REQ-RENDER-026).
//! - `shape_scene_samples_the_chart`: each display tile's sample is the chart's point at the chart cell its display
//!   point falls in, through the projection, before `C`'s fold; Hammer's ellipse's outside, and only it, is no system;
//!   the spherical map's scene flags the redundant hemisphere and the exponential cap's, in one hemisphere, flags
//!   nothing (REQ-CHART-002, REQ-RENDER-026).
//! - `shape_scene_binary_writes_the_render`: `golden_harness --scene` writes each scene's render.
//!
//! Each test registers its negative control (R-176).

use std::process::Command;

use kernel::chart::shape_sphere::{Cap, Projection, ShapeMap};
use kernel::chart::SystemImage;
use render::present::Rgb;
use validation::golden_scene::{encode_image, tie_margin};
use validation::gpu::GpuHarness;
use validation::negative_control;
use validation::shape_scene::{cell_centre, scene, ShapeScene, CAP_GRID, CHART, DISPLAY, NAMES};

/// The largest per-channel difference, linear RGB, between a render and its CPU twin, as the other scenes' tests allow.
const TOL: f64 = 1e-5;

/// The least distance, in 8-bit steps, of any scene's twin from a rounding tie, as the other scenes' tests require.
const MIN_TIE_MARGIN: f64 = 0.01;

/// The least `|n.x|`, `|n.y|` and, where the flag is drawn, `|n.z|` of any sample: far above the f32 read's error, so
/// no backend's rounding flips a quadrant or a hemisphere.
const MIN_SIGN_MARGIN: f32 = 1e-3;

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

fn named(name: &str) -> ShapeScene {
    scene(name).unwrap_or_else(|e| panic!("{e}"))
}

fn render(h: &GpuHarness, s: &ShapeScene) -> Vec<Rgb> {
    s.scene
        .render(h.device(), h.queue())
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .map(|p| [p[0], p[1], p[2]].map(f64::from))
        .collect()
}

/// Checks that `image` is `want`, pixel by pixel, within [`TOL`].
fn check_image(s: &ShapeScene, image: &[Rgb], want: &[Rgb]) {
    let width = s.scene.size().0 as usize;
    assert_eq!(
        image.len(),
        want.len(),
        "`{}`: the render's size",
        s.scene.name
    );
    for (k, (got, want)) in image.iter().zip(want).enumerate() {
        assert!(
            got.iter().zip(want).all(|(a, b)| (a - b).abs() <= TOL),
            "pixel ({}, {}) of `{}` is {got:?}, not its CPU twin's {want:?}",
            k % width,
            k / width,
            s.scene.name
        );
    }
}

/// Every sample of `s` that is a system reads its `n` at least [`MIN_SIGN_MARGIN`] from each sign the occupant reads.
fn check_sign_margins(s: &ShapeScene) {
    let shapes = s.shapes();
    for i in 0..s.scene.context.grid.sample_count() {
        if s.scene.set.forbidden(i) {
            continue;
        }
        let n = shapes[i as usize];
        let read = if s.fold { 3 } else { 2 };
        for (k, c) in n.iter().enumerate().take(read) {
            assert!(
                c.abs() >= MIN_SIGN_MARGIN,
                "`{}` sample {i}'s n[{k}] = {c} lies near its sign's flip",
                s.scene.name
            );
        }
    }
}

#[test]
fn shape_scene_renders_its_twin_clear_of_ties() {
    let h = gpu();
    for name in NAMES {
        let s = named(name);
        let want = s.expected();
        check_image(&s, &render(&h, &s), &want);
        let margin = tie_margin(&want);
        eprintln!("{name}: tie margin {margin:.4} steps");
        assert!(
            margin >= MIN_TIE_MARGIN,
            "`{name}`'s twin lies {margin} steps from a rounding tie"
        );
        check_sign_margins(&s);
    }
}

negative_control!(
    shape_scene_renders_its_twin_clear_of_ties,
    "the spherical map's render without its redundancy flag is not its twin",
    expected = "not its CPU twin",
    {
        let h = gpu();
        let s = named("shape_equirectangular");
        let mut bare = named("shape_equirectangular");
        bare.fold = false;
        check_image(&s, &render(&h, &s), &bare.expected());
    }
);

/// The chart point `s`'s display tile `(col, row)` shows: its display point through the projection, the chart cell
/// holding it, that cell's centre; `None` where it shows none.
fn shown(s: &ShapeScene, projection: Projection, col: u32, row: u32) -> Option<[f64; 2]> {
    let (display, cells) = match s.chart.map {
        ShapeMap::Spherical => (DISPLAY, CHART),
        ShapeMap::Exponential => (CAP_GRID, CAP_GRID),
    };
    let centre = |i: u32, n: u32| (f64::from(i) + 0.5) / f64::from(n);
    let [x, y] = projection.chart_point(centre(col, display[0]), centre(row, display[1]))?;
    let cell = |c: f64, n: u32| centre(((c * f64::from(n)).floor() as u32).min(n - 1), n);
    Some([cell(x, cells[0]), cell(y, cells[1])])
}

/// Each display tile of `s` under `projection` holds the chart's point at the chart cell it shows, `Φ`'s own, before
/// `C`; a tile showing none is no system; `s` flags the redundancy where, and only where, the chart is n-to-1 over its
/// φ hemispheres.
fn check_samples(s: &ShapeScene, projection: Projection) {
    let grid = s.scene.context.grid;
    let shapes = s.shapes();
    let mut lower = 0;
    for row in 0..grid.quads[1] {
        for col in 0..grid.quads[0] {
            let i = row * grid.quads[0] + col;
            let Some([cs, ct]) = shown(s, projection, col, row) else {
                assert!(
                    s.scene.set.forbidden(i),
                    "`{}` tile ({col}, {row}) shows no chart point",
                    s.scene.name
                );
                continue;
            };
            assert!(
                !s.scene.set.forbidden(i),
                "`{}` tile ({col}, {row}) is marked no system",
                s.scene.name
            );
            let want = s.chart.point::<f64>(cs, ct);
            let got = shapes[i as usize];
            assert!(
                (0..3).all(|k| (f64::from(got[k]) - want[k]).abs() < 1e-5),
                "`{}` tile ({col}, {row}) reads n = {got:?}, not the chart's point {want:?}",
                s.scene.name
            );
            lower += usize::from(got[2] < 0.0);
        }
    }
    match s.chart.map {
        ShapeMap::Spherical => {
            assert_eq!(s.chart.system_image(), SystemImage::NToOne(2));
            assert!(
                s.fold && lower > 0,
                "`{}` draws no flagged hemisphere",
                s.scene.name
            );
        }
        ShapeMap::Exponential => {
            assert_eq!(s.chart.cap(), Cap::Upper);
            assert_eq!(s.chart.system_image(), SystemImage::Bijective);
            assert!(s
                .chart
                .label()
                .quotient
                .starts_with("one hemisphere, w ≥ 0"));
            assert!(
                !s.fold && lower == 0,
                "`{}` is not one hemisphere",
                s.scene.name
            );
        }
    }
}

#[test]
fn shape_scene_samples_the_chart() {
    check_samples(&named("shape_equirectangular"), Projection::Equirectangular);
    check_samples(&named("shape_equal_area"), Projection::EqualArea);
    check_samples(&named("shape_one_hemisphere"), Projection::Equirectangular);
    let hammer = named("shape_equal_area");
    let outside = (0..hammer.scene.context.grid.sample_count())
        .filter(|&i| hammer.scene.set.forbidden(i))
        .count();
    assert!(outside > 0, "Hammer's scene shows its whole square");
    for (name, map, size) in [
        ("shape_equirectangular", ShapeMap::Spherical, DISPLAY),
        ("shape_equal_area", ShapeMap::Spherical, DISPLAY),
        ("shape_one_hemisphere", ShapeMap::Exponential, CAP_GRID),
    ] {
        let s = named(name);
        assert_eq!(s.chart.map, map, "`{name}` draws the wrong map");
        assert_eq!(
            s.scene.context.grid.quads, size,
            "`{name}` has the wrong display grid"
        );
    }
    for (c, want) in [(0.0, 0.125), (0.25, 0.375), (0.999, 0.875), (1.0, 0.875)] {
        assert_eq!(cell_centre(c, 4), want, "c = {c} is not in its chart cell");
    }
}

negative_control!(
    shape_scene_samples_the_chart,
    "a sample holding C's canonical state, the lower hemisphere folded onto the upper, is not the chart's point",
    expected = "not the chart's point",
    {
        let mut s = named("shape_equirectangular");
        let [cs, ct] = shown(&s, Projection::Equirectangular, 0, 0).expect("the corner shows a chart point");
        let d = kernel::chart::Chart::decode::<kernel::generated::links::Defaults, f64>(&s.chart, cs, ct);
        s.scene.set.sample(0).r(d.ic.r.map(|p| p.map(|c| c as f32)));
        check_samples(&s, Projection::Equirectangular);
    }
);

fn harness(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_golden_harness"))
        .args(args)
        .output()
        .expect("run golden_harness")
}

/// Checks that `golden_harness --scene name` writes scene `against`'s render in the image format.
fn check_binary(name: &str, against: &str) {
    let out = harness(&["--scene", name]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let h = gpu();
    let s = named(against);
    let pixels = s
        .scene
        .render(h.device(), h.queue())
        .unwrap_or_else(|e| panic!("{e}"));
    let (w, hgt) = s.scene.size();
    assert!(
        out.stdout == encode_image(w, hgt, &pixels),
        "the binary's `{name}` is not `{against}`'s render"
    );
}

#[test]
fn shape_scene_binary_writes_the_render() {
    for name in NAMES {
        check_binary(name, name);
    }
}

negative_control!(
    shape_scene_binary_writes_the_render,
    "the binary's equirectangular scene is not the equal-area scene's render",
    expected = "is not `shape_equal_area`'s render",
    check_binary("shape_equirectangular", "shape_equal_area")
);
