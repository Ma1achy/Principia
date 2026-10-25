//! The shape-sphere scenes (`validation::shape_scene`; TASK-M2-28), rendered through the render harness and held to
//! their CPU twins and to the chart:
//! - `shape_scene_renders_its_twin_clear_of_ties`: every scene renders its CPU twin, far enough from every rounding tie
//!   that one reference holds on every backend, each sample's octant far from its signs' flips (REQ-CHART-002,
//!   REQ-RENDER-026).
//! - `shape_scene_samples_the_chart`: each display tile's sample of a chart scene is the chart's signed point at the
//!   chart cell its display point falls in, through the projection, the decode's own state with nothing folded; Hammer's
//!   ellipse's outside, and only it, is no system; every chart scene draws both hemispheres, bijective, and flags
//!   nothing (R-544; REQ-CHART-002, REQ-RENDER-026).
//! - `shape_scene_controls_are_wrong`: the one-hemisphere control draws no sample below the equator, and the flagged
//!   control's twin is the spherical scene's except on the lower half's stripes, where it is black (REQ-CHART-002).
//! - `shape_scene_binary_writes_the_render`: `golden_harness --scene` writes each scene's render.
//!
//! Each test registers its negative control (R-176).

use std::process::Command;

use kernel::chart::shape_sphere::Projection;
use kernel::chart::SystemImage;
use render::present::Rgb;
use validation::golden_scene::{encode_image, tie_margin};
use validation::gpu::GpuHarness;
use validation::negative_control;
use validation::shape_scene::{
    cell_centre, scene, shown, Draw, ShapeScene, CAP_GRID, CHART, DISPLAY, NAMES,
};

/// The largest per-channel difference, linear RGB, between a render and its CPU twin, as the other scenes' tests allow.
const TOL: f64 = 1e-5;

/// The least distance, in 8-bit steps, of any scene's twin from a rounding tie, as the other scenes' tests require.
const MIN_TIE_MARGIN: f64 = 0.01;

/// The least `|n.x|`, `|n.y|` and `|n.z|` of any sample: far above the f32 read's error, so no backend's rounding
/// flips a quadrant or a hemisphere.
const MIN_SIGN_MARGIN: f32 = 1e-3;

/// The chart scenes, each the chart's own render, with the projection it is drawn through.
const CHART_SCENES: [(&str, Projection); 3] = [
    ("shape_spherical", Projection::Equirectangular),
    ("shape_equal_area", Projection::EqualArea),
    ("shape_exponential_straddling", Projection::Equirectangular),
];

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
        for (k, c) in n.iter().enumerate() {
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
    "the spherical map's render held to the flagged control's twin, its lower half striped, is not its twin",
    expected = "not its CPU twin",
    {
        let h = gpu();
        let s = named("shape_spherical");
        check_image(&s, &render(&h, &s), &named("shape_control_flagged").expected());
    }
);

/// Each display tile of the chart scene `s` under `projection` holds the chart's signed point at the chart cell it
/// shows, the decode's own state; a tile showing none is no system; `s` draws both hemispheres, bijective, and flags
/// nothing.
fn check_samples(s: &ShapeScene, projection: Projection) {
    let grid = s.scene.context.grid;
    let (cells, display) = s.grids();
    assert_eq!(grid.quads, display, "`{}`'s display grid", s.scene.name);
    let shapes = s.shapes();
    let (mut lower, mut upper) = (0, 0);
    for row in 0..grid.quads[1] {
        for col in 0..grid.quads[0] {
            let i = row * grid.quads[0] + col;
            let Some([cs, ct]) = shown(projection, Draw::Chart, display, cells, col, row) else {
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
            upper += usize::from(got[2] > 0.0);
        }
    }
    assert_eq!(s.chart.system_image(), SystemImage::Bijective);
    assert!(
        s.draw == Draw::Chart && lower > 0 && upper > 0,
        "`{}` does not draw both hemispheres unflagged ({upper} above, {lower} below)",
        s.scene.name
    );
}

#[test]
fn shape_scene_samples_the_chart() {
    for (name, projection) in CHART_SCENES {
        let s = named(name);
        assert_eq!(s.projection, projection, "`{name}`'s projection");
        check_samples(&s, projection);
    }
    let hammer = named("shape_equal_area");
    let outside = (0..hammer.scene.context.grid.sample_count())
        .filter(|&i| hammer.scene.set.forbidden(i))
        .count();
    assert!(outside > 0, "Hammer's scene shows its whole square");
    assert_eq!(named("shape_spherical").grids(), (CHART, DISPLAY));
    assert_eq!(
        named("shape_exponential_straddling").grids(),
        (CAP_GRID, CAP_GRID)
    );
    for (c, want) in [(0.0, 0.125), (0.25, 0.375), (0.999, 0.875), (1.0, 0.875)] {
        assert_eq!(cell_centre(c, 4), want, "c = {c} is not in its chart cell");
    }
}

negative_control!(
    shape_scene_samples_the_chart,
    "a sample holding R-141's folded state, a lower-hemisphere system mirrored onto the upper, is not the chart's point",
    expected = "not the chart's point",
    {
        let mut s = named("shape_spherical");
        let [cs, ct] = shown(Projection::Equirectangular, Draw::Chart, DISPLAY, CHART, 0, 0)
            .expect("the corner shows a chart point");
        assert!(ct < 0.5, "the corner tile is not below the equator");
        let d = kernel::chart::Chart::decode::<kernel::generated::links::Defaults, f64>(&s.chart, cs, ct);
        let mirrored = d.ic.r.map(|p| [p[0] as f32, -p[1] as f32]);
        s.scene.set.sample(0).r(mirrored);
        check_samples(&s, Projection::Equirectangular);
    }
);

/// The one-hemisphere control `one` draws no sample below the equator; the flagged control `flagged`'s twin is
/// `whole`'s twin except on the lower half's stripes, where it is black, and there is at least one such pixel.
fn check_controls(one: &ShapeScene, flagged: &ShapeScene, whole: &ShapeScene) {
    let below = one.shapes().iter().filter(|n| n[2] < 0.0).count();
    assert_eq!(
        below, 0,
        "the one-hemisphere control draws {below} samples below the equator"
    );
    let (want, got) = (whole.expected(), flagged.expected());
    let grid = whole.scene.context.grid;
    let shapes = whole.shapes();
    let width = whole.scene.size().0;
    let mut striped = 0;
    for (k, (w, g)) in want.iter().zip(&got).enumerate() {
        let (x, y) = (k as u32 % width, k as u32 / width);
        let lower = shapes[grid.cell(x, y).sample as usize][2] < 0.0;
        if lower && ShapeScene::stripe(x, y) {
            striped += 1;
            assert_eq!(
                *g, [0.0; 3],
                "the flagged control's ({x}, {y}) is not striped"
            );
        } else {
            assert_eq!(g, w, "the flagged control's ({x}, {y}) is not the chart's");
        }
    }
    assert!(striped > 0, "the flagged control stripes nothing");
}

#[test]
fn shape_scene_controls_are_wrong() {
    check_controls(
        &named("shape_control_one_hemisphere"),
        &named("shape_control_flagged"),
        &named("shape_spherical"),
    );
}

negative_control!(
    shape_scene_controls_are_wrong,
    "the whole-sphere scene taken for the one-hemisphere control draws the lower hemisphere, so it must fail",
    expected = "samples below the equator",
    {
        check_controls(
            &named("shape_spherical"),
            &named("shape_control_flagged"),
            &named("shape_spherical"),
        );
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
    "the binary's spherical scene is not the equal-area scene's render",
    expected = "is not `shape_equal_area`'s render",
    check_binary("shape_spherical", "shape_equal_area")
);
