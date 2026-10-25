//! The shape-sphere scenes (`validation::shape_scene`; TASK-M2-28), rendered through the render harness and held to
//! their CPU twins and to the chart:
//! - `shape_scene_renders_its_twin_clear_of_ties`: every scene renders its CPU twin, far enough from every rounding tie
//!   that one reference holds on every backend, each sample's octant far from its signs' flips (REQ-CHART-002,
//!   REQ-RENDER-026).
//! - `shape_scene_samples_the_chart`: each display tile's sample of a chart scene is the chart's signed point at the
//!   chart cell its display point falls in, through the projection, the decode's own state with nothing folded; Hammer's
//!   ellipse's outside, and only it, is display margin, no sample in the forbidden lane, each tile centre clear of the
//!   ellipse's edge; every chart scene draws both hemispheres, bijective, and flags nothing (R-544; REQ-CHART-002,
//!   REQ-RENDER-026).
//! - `shape_scene_octant_palette`, `_deutan`, `_protan`, `_tritan`: every pair of the eight octant colours lies at least
//!   R-429's τ apart under normal vision and under each of R-383's deutan, protan and tritan simulations.
//! - `shape_scene_octant_palette_background`, `_stripe`, `_lighter`: under each, every octant colour lies at least 0.4
//!   from the canvas background and from the stripes, and the upper colours are all lighter than the lower.
//! - `shape_scene_controls_are_wrong`: the one-hemisphere control draws no sample below the equator, and the flagged
//!   control's twin is the spherical scene's except on the lower half's stripes, where it is black (REQ-CHART-002).
//! - `shape_scene_class_signs`, `shape_scene_twin_on_the_equator`: the twin reads each sign of `n` strictly, as the
//!   occupant does, a zero not below it; a sample on the equator is not striped by the flagged control.
//! - `shape_scene_binary_writes_the_render`: `golden_harness --scene` writes each scene's render.
//!
//! Each test registers its negative control (R-176).

use std::process::Command;

use kernel::chart::shape_sphere::Projection;
use kernel::chart::SystemImage;
use render::display::cvd::{self, CvdMode};
use render::present::{self, Rgb};
use validation::golden_scene::{encode_image, tie_margin};
use validation::gpu::GpuHarness;
use validation::negative_control;
use validation::shape_scene::{
    background, cell_centre, octant_colour, scene, shown, Draw, ShapeScene, CAP_GRID, CHART,
    DISPLAY, NAMES, STRIPE,
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
        if !s.chart_point[i as usize] {
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
                    !s.chart_point[i as usize],
                    "`{}` tile ({col}, {row}) shows no chart point",
                    s.scene.name
                );
                continue;
            };
            assert!(
                s.chart_point[i as usize],
                "`{}` tile ({col}, {row}) is marked margin",
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
    let outside = hammer.chart_point.iter().filter(|&&c| !c).count();
    assert!(outside > 0, "Hammer's scene shows its whole square");
    // No sample of any scene is in the forbidden lane: the margin is not a classified not-a-system sample (REQ-COL-102).
    for name in NAMES {
        let s = named(name);
        let grid = s.scene.context.grid;
        assert!(
            (0..grid.sample_count()).all(|i| !s.scene.set.forbidden(i)),
            "`{name}` marks a sample forbidden"
        );
    }
    // Every display tile's centre lies at least 10⁻⁴ from the ellipse's edge, so the occupant's f32 test and the CPU's
    // f64 one agree on every tile.
    let centre = |i: u32, n: u32| (f64::from(i) + 0.5) / f64::from(n);
    for row in 0..DISPLAY[1] {
        for col in 0..DISPLAY[0] {
            let (h, k) = (
                2.0 * centre(col, DISPLAY[0]) - 1.0,
                2.0 * centre(row, DISPLAY[1]) - 1.0,
            );
            let q = h * h + k * k;
            assert!(
                (q - 1.0).abs() >= 1e-4,
                "tile ({col}, {row}) lies at q = {q}, on the ellipse's edge"
            );
        }
    }
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

/// `class` reads each sign strictly, as the occupant's `select(…, n.x < 0.0)` does: a zero of either sign is not below
/// it, and the least negative value is.
fn check_class(class: fn([f32; 3]) -> u32) {
    for (n, want) in [
        ([0.0, 0.0, 0.0], 0),
        ([-0.0, -0.0, -0.0], 0),
        ([-f32::MIN_POSITIVE, 0.0, 0.0], 1),
        ([0.0, -f32::MIN_POSITIVE, 0.0], 2),
        ([0.0, 0.0, -f32::MIN_POSITIVE], 4),
        ([-1.0, -1.0, -1.0], 7),
    ] {
        assert_eq!(class(n), want, "the class of {n:?} is not the occupant's");
    }
}

#[test]
fn shape_scene_class_signs() {
    check_class(ShapeScene::class);
}

negative_control!(
    shape_scene_class_signs,
    "a class that reads a zero as below its sign must fail",
    expected = "is not the occupant's",
    check_class(|n| u32::from(n[0] <= 0.0)
        + 2 * u32::from(n[1] <= 0.0)
        + 4 * u32::from(n[2] <= 0.0))
);

/// R-429's τ: the OKLab distance from `#FF00FF` to `#E034C6`, 0.101, the corpus's bar for two colours told apart.
fn tau() -> f64 {
    distance(
        present::srgb8([0xff, 0x00, 0xff]),
        present::srgb8([0xe0, 0x34, 0xc6]),
    )
}

/// The OKLab distance between two linear colours.
fn distance(a: Rgb, b: Rgb) -> f64 {
    let (p, q) = (present::linear_to_oklab(a), present::linear_to_oklab(b));
    ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2)).sqrt()
}

/// Normal vision, then R-383's three dichromat simulations.
const VISION: [Option<CvdMode>; 4] = [
    None,
    Some(CvdMode::Deuteranopia),
    Some(CvdMode::Protanopia),
    Some(CvdMode::Tritanopia),
];

/// What the screen shows of `c` under `mode`: simulated and clamped to [0, 1], as the pass writes.
fn seen(mode: Option<CvdMode>, c: Rgb) -> Rgb {
    mode.map_or(c, |m| cvd::simulate(m, c).map(|x| x.clamp(0.0, 1.0)))
}

/// How a failure names the vision it was found under.
fn vision(mode: Option<CvdMode>) -> String {
    mode.map_or("normal vision".to_owned(), |m| format!("{m:?}"))
}

/// The eight octant colours the scenes draw, octant k at index k.
fn octants() -> [Rgb; 8] {
    std::array::from_fn(|k| octant_colour(k as u32))
}

/// The octant colours with octant `k`'s replaced by the 8-bit code `code`: a control palette.
#[cfg(feature = "controls")]
fn with(k: usize, code: [u8; 3]) -> [Rgb; 8] {
    let mut p = octants();
    p[k] = present::srgb8(code);
    p
}

/// The palette criterion (gui review 5644431015, G3, applied per R-369), its first part: under `mode`, every pair of
/// the eight colours lies at least R-429's τ apart.
fn check_distinct(mode: Option<CvdMode>, palette: [Rgb; 8]) {
    let s = palette.map(|c| seen(mode, c));
    for k in 0..8 {
        for j in 0..k {
            let d = distance(s[j], s[k]);
            assert!(
                d >= tau(),
                "{}: octants {j} and {k} lie {d:.4} apart, under τ = {:.4}",
                vision(mode),
                tau()
            );
        }
    }
}

/// The palette criterion, its second part: under normal vision and each simulation, every colour lies at least 0.4
/// from the canvas background.
fn check_background(palette: [Rgb; 8]) {
    for mode in VISION {
        for (k, &c) in palette.iter().enumerate() {
            let d = distance(seen(mode, c), seen(mode, background()));
            assert!(
                d >= 0.4,
                "{}: octant {k} lies {d:.4} from the canvas background",
                vision(mode)
            );
        }
    }
}

/// The palette criterion, its third part: under normal vision and each simulation, every colour lies at least 0.4
/// from the stripe colour `stripe`.
fn check_stripe(palette: [Rgb; 8], stripe: Rgb) {
    for mode in VISION {
        for (k, &c) in palette.iter().enumerate() {
            let d = distance(seen(mode, c), seen(mode, stripe));
            assert!(
                d >= 0.4,
                "{}: octant {k} lies {d:.4} from the stripe colour",
                vision(mode)
            );
        }
    }
}

/// The palette criterion, its last part: every upper colour (octants 0–3) is lighter in OKLab than every lower one.
fn check_lighter(palette: [Rgb; 8]) {
    let l = |k: usize| present::linear_to_oklab(palette[k])[0];
    let (low_up, high_down) = (
        (0..4).map(l).fold(1.0, f64::min),
        (4..8).map(l).fold(0.0, f64::max),
    );
    assert!(
        low_up > high_down,
        "the upper colours are not all lighter than the lower ones ({low_up:.4} against {high_down:.4})"
    );
}

#[test]
fn shape_scene_octant_palette() {
    check_distinct(None, octants());
}

negative_control!(
    shape_scene_octant_palette,
    "the same-lightness hue wheel at (8k + 7)/64 turns, 9899c04f's palette, fails under normal vision (octants 0 and 1 0.0918 apart), so it must fail",
    expected = "normal vision: octants 0 and 1",
    check_distinct(None, std::array::from_fn(|k| present::hue_wheel(f64::from(8 * k as u32 + 7) / 64.0)))
);

#[test]
fn shape_scene_octant_palette_deutan() {
    check_distinct(Some(CvdMode::Deuteranopia), octants());
}

negative_control!(
    shape_scene_octant_palette_deutan,
    "octant 6 as #B400FF passes every other check but lies 0.0034 from another octant under deutan simulation, so it must fail",
    expected = "Deuteranopia: octants",
    check_distinct(Some(CvdMode::Deuteranopia), with(6, [0xb4, 0x00, 0xff]))
);

#[test]
fn shape_scene_octant_palette_protan() {
    check_distinct(Some(CvdMode::Protanopia), octants());
}

negative_control!(
    shape_scene_octant_palette_protan,
    "octant 6 as #8769B4 passes every other check but lies 0.057 from another octant under protan simulation, so it must fail",
    expected = "Protanopia: octants",
    check_distinct(Some(CvdMode::Protanopia), with(6, [0x87, 0x69, 0xb4]))
);

#[test]
fn shape_scene_octant_palette_tritan() {
    check_distinct(Some(CvdMode::Tritanopia), octants());
}

negative_control!(
    shape_scene_octant_palette_tritan,
    "octant 6 as #4B873C passes every other check but lies 0.078 from another octant under tritan simulation, so it must fail",
    expected = "Tritanopia: octants",
    check_distinct(Some(CvdMode::Tritanopia), with(6, [0x4b, 0x87, 0x3c]))
);

#[test]
fn shape_scene_octant_palette_background() {
    check_background(octants());
}

negative_control!(
    shape_scene_octant_palette_background,
    "octant 7 as #003CC3 passes every other check but lies 0.31–0.36 from the canvas background, so it must fail",
    expected = "from the canvas background",
    check_background(with(7, [0x00, 0x3c, 0xc3]))
);

#[test]
fn shape_scene_octant_palette_stripe() {
    check_stripe(octants(), STRIPE);
}

negative_control!(
    shape_scene_octant_palette_stripe,
    "white stripes, 9899c04f's, lie under 0.4 from octant 0's #F0FFD2, while the octants pass every other check, so they must fail",
    expected = "from the stripe colour",
    check_stripe(octants(), [1.0; 3])
);

#[test]
fn shape_scene_octant_palette_lighter() {
    check_lighter(octants());
}

negative_control!(
    shape_scene_octant_palette_lighter,
    "the octant colours with octants 3 and 7 swapped keep every distance but put a lower colour above an upper one, so they must fail",
    expected = "not all lighter",
    check_lighter({
        let mut p = octants();
        p.swap(3, 7);
        p
    })
);

/// The flagged control with its sample 0 collinear on the x axis, so its read `n.z` is a zero: `want` gives the twin,
/// and the render must be it, the zero not striped as below the equator.
fn check_equator_sample(want: fn(&ShapeScene) -> Vec<Rgb>) {
    let h = gpu();
    let mut s = named("shape_control_flagged");
    s.scene
        .set
        .sample(0)
        .r([[-1.0, 0.0], [0.25, 0.0], [0.75, 0.0]]);
    let n = s.shapes()[0];
    assert_eq!(n[2], 0.0, "sample 0's n = {n:?} is not on the equator");
    check_image(&s, &render(&h, &s), &want(&s));
}

#[test]
fn shape_scene_twin_on_the_equator() {
    check_equator_sample(ShapeScene::expected);
}

negative_control!(
    shape_scene_twin_on_the_equator,
    "a twin that stripes the equator's zero as below it is not the render",
    expected = "not its CPU twin",
    check_equator_sample(|s| {
        let mut image = s.expected();
        let grid = s.scene.context.grid;
        let width = s.scene.size().0;
        for (k, px) in image.iter_mut().enumerate() {
            let (x, y) = (k as u32 % width, k as u32 / width);
            if grid.cell(x, y).sample == 0 && ShapeScene::stripe(x, y) {
                *px = STRIPE;
            }
        }
        image
    })
);

/// The one-hemisphere control `one` draws no sample below the equator and spans the upper hemisphere, its samples
/// from within 0.1 of the pole `w = 1` to within 0.1 of the equator; the flagged control `flagged`'s twin is `whole`'s
/// twin except on the lower half's stripes, where it is [`STRIPE`], and there is at least one such pixel.
fn check_controls(one: &ShapeScene, flagged: &ShapeScene, whole: &ShapeScene) {
    let shapes = one.shapes();
    let below = shapes.iter().filter(|n| n[2] < 0.0).count();
    assert_eq!(
        below, 0,
        "the one-hemisphere control draws {below} samples below the equator"
    );
    let w = shapes.iter().map(|n| n[2]);
    let (low, high) = (w.clone().fold(1.0, f32::min), w.fold(0.0, f32::max));
    assert!(
        low < 0.1 && high > 0.9,
        "the one-hemisphere control spans w ∈ [{low}, {high}], not pole to equator"
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
                *g, STRIPE,
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
