//! The structural scenes (`validation::structural_scene`; TASK-M1-13), rendered through the render harness and held
//! to their CPU twins:
//! - REQ-TOOL-026: each structural view renders from a synthetic `RenderQuad` set, its quads' values the CPU structs'
//!   (`structural_scene_views_*`); with `has_ensemble()` false the spread view draws the hatch everywhere
//!   (`structural_scene_spread_*`); the fallback tint and the pending hatch mark exactly the quads their predicates
//!   pick, the hatch never in `debug_invalid`'s colours (`structural_scene_overlays_*`);
//! - REQ-RENDER-024: the boundary lines measure one pixel width on the depth-3 and depth-20 sets at quad sizes 16 and
//!   64 px, and the thresholded-uv control does not (`structural_scene_boundary_width_*`);
//! - every scene renders its CPU twin, far enough from every rounding tie that one reference holds on every backend
//!   (`structural_scene_renders_*`), and the binary writes it (`structural_scene_binary_*`).
//!
//! Each test registers its negative control (R-176).

use std::process::Command;

use engine::synthetic::structural_record;
use render::colour::combine::MID_GREY_L;
use render::present::{self, Rgb};
use render::structural::{self as mirror, QuadMeta};
use validation::golden_scene::{encode_image, tie_margin};
use validation::gpu::GpuHarness;
use validation::negative_control;
use validation::structural_scene::{line_width, scene, StructuralScene, NAMES};

/// The largest per-channel difference, linear RGB, between a render and its CPU twin, as `golden_scene`'s tests allow.
const TOL: f64 = 1e-5;

/// The least distance, in 8-bit steps, of any scene's twin from a rounding tie, as `golden_scene`'s tests require.
const MIN_TIE_MARGIN: f64 = 0.01;

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

fn named(name: &str) -> StructuralScene {
    scene(name).unwrap_or_else(|e| panic!("{e}"))
}

fn render(h: &GpuHarness, s: &StructuralScene) -> Vec<Rgb> {
    s.render(h.device(), h.queue())
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .map(|p| {
            assert_eq!(p[3], 1.0, "`{}`: alpha", s.name);
            [p[0], p[1], p[2]].map(f64::from)
        })
        .collect()
}

fn twin(s: &StructuralScene) -> Vec<Rgb> {
    s.expected().unwrap_or_else(|e| panic!("{e}"))
}

fn near(a: Rgb, b: Rgb) -> bool {
    a.iter().zip(&b).all(|(x, y)| (x - y).abs() <= TOL)
}

/// Checks that `image` is `want`, pixel by pixel, within [`TOL`].
fn check_image(s: &StructuralScene, image: &[Rgb], want: &[Rgb], what: &str) {
    let width = s.size().0 as usize;
    assert_eq!(image.len(), want.len(), "`{}`: the render's size", s.name);
    for (k, (got, want)) in image.iter().zip(want).enumerate() {
        assert!(
            near(*got, *want),
            "pixel ({}, {}) of `{}` is {got:?}, not {what} {want:?}",
            k % width,
            k / width,
            s.name
        );
    }
}

#[test]
fn structural_scene_renders_its_twin_clear_of_ties() {
    let h = gpu();
    for name in NAMES {
        let s = named(name);
        let want = twin(&s);
        check_image(&s, &render(&h, &s), &want, "its CPU twin");
        let margin = tie_margin(&want);
        eprintln!("{name}: tie margin {margin:.4} steps");
        assert!(
            margin >= MIN_TIE_MARGIN,
            "`{name}`'s twin lies {margin} steps from a rounding tie"
        );
    }
}

negative_control!(
    structural_scene_renders_its_twin_clear_of_ties,
    "the tint scene's render against the pending scene's twin differs",
    expected = "not its CPU twin",
    {
        let h = gpu();
        let (tint, pending) = (named("fallback_tint"), named("pending_hatch"));
        check_image(&tint, &render(&h, &tint), &twin(&pending), "its CPU twin");
    }
);

/// The structural set's depths, quad by quad: 3 and 20 alternately.
fn depth(q: u32) -> u32 {
    [3, 20][q as usize % 2]
}

/// Checks that quad `q` of `s` holds [`structural_record`]'s record and its depth, read back from the set's CPU words.
fn check_quad(s: &StructuralScene, q: u32, depth: u32) {
    let r = structural_record(q);
    let want = QuadMeta {
        depth,
        state: r.state,
        coherence: r.coherence,
        impurity: r.impurity,
        spread: r.spread,
        suspect_frac: r.suspect,
        priority: r.priority,
        ancestor_gap: r.ancestor_gap,
        cache_age: r.cache_age,
        dominant_outcome: r.dominant_outcome,
    };
    assert_eq!(
        s.quad(q),
        want,
        "`{}` quad {q}: the CPU struct's values",
        s.name
    );
}

/// Checks that every pixel of each quad of the view scene `name` shows the view of that quad's CPU record, and the
/// quads show what the record says: their states each a colour, the depths 3 and 20 the ramp's ends.
fn check_view(h: &GpuHarness, name: &str) {
    let s = named(name);
    let id = s.view.unwrap_or_else(|| panic!("`{name}` has no view"));
    let image = render(h, &s);
    let (width, height) = s.size();
    let range = s.u_range();
    for q in 0..s.context.grid.quad_count() {
        check_quad(&s, q, depth(q));
    }
    for y in 0..height {
        for x in 0..width {
            let cell = s.context.grid.cell(x, y);
            let frag = [f64::from(x) + 0.5, f64::from(y) + 0.5];
            let want = mirror::view(
                id,
                &s.quad(cell.quad),
                frag,
                s.context.grid.e >= 1,
                mirror::range_auto(id),
                range.unwrap_or([0.0, 1.0]),
            )
            .unwrap_or_else(|| panic!("`{id}` is no view"));
            let got = image[(y * width + x) as usize];
            assert!(
                near(got, want),
                "`{name}` pixel ({x}, {y}), quad {}: {got:?}, not its record's view {want:?}",
                cell.quad
            );
        }
    }
}

#[test]
fn structural_scene_views_render_the_cpu_structs() {
    let h = gpu();
    for id in mirror::VIEWS {
        check_view(&h, id);
    }
    check_view(&h, "s_spread_ensemble");
    // The depth view spans the two depths: depth 3 at the ramp's start, depth 20 at its end.
    let s = named("s_depth");
    assert_eq!(s.u_range(), Some([3.0, 20.0]), "the depths measured");
    let image = render(&h, &s);
    let at = |q: u32| {
        let (width, height) = s.size();
        (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .find(|&(x, y)| s.context.grid.cell(x, y).quad == q)
            .map(|(x, y)| image[(y * width + x) as usize])
            .unwrap_or_else(|| panic!("quad {q} has no pixel"))
    };
    assert!(near(at(0), present::ramp_viridis(0.0)), "depth 3");
    assert!(near(at(1), present::ramp_viridis(1.0)), "depth 20");
    // The state view: each state its Okabe–Ito colour, the five distinct.
    let s = named("s_state");
    let colours: Vec<Rgb> = (0..5).map(|k| present::dbg_cat(k, 5)).collect();
    for (i, a) in colours.iter().enumerate() {
        for b in &colours[i + 1..] {
            assert!(!near(*a, *b), "two states share a colour");
        }
    }
    assert_eq!(s.quad(1).state, 1, "quad 1 is pending");
}

negative_control!(
    structural_scene_views_render_the_cpu_structs,
    "a quad whose CPU struct is changed after the render no longer matches the image",
    expected = "not its record's view",
    {
        let h = gpu();
        let s = named("s_impurity");
        let image = render(&h, &s);
        let mut changed = named("s_impurity");
        changed
            .set
            .quad(3)
            .f32("outcome_impurity", 0.95)
            .expect("a member");
        let (width, height) = s.size();
        for y in 0..height {
            for x in 0..width {
                let cell = changed.context.grid.cell(x, y);
                let frag = [f64::from(x) + 0.5, f64::from(y) + 0.5];
                let want = mirror::view(
                    "s_impurity",
                    &changed.quad(cell.quad),
                    frag,
                    false,
                    false,
                    [0.0, 1.0],
                )
                .expect("a view");
                let got = image[(y * width + x) as usize];
                assert!(
                    near(got, want),
                    "pixel ({x}, {y}): {got:?}, not its record's view {want:?}"
                );
            }
        }
    }
);

/// Checks that every pixel of `image`, from scene `s`, is the hatch, `debug_invalid`, iff `hatched`.
fn check_hatched(s: &StructuralScene, image: &[Rgb], hatched: bool) {
    let width = s.size().0;
    for (k, got) in image.iter().enumerate() {
        let (x, y) = (k as u32 % width, k as u32 / width);
        let hatch = present::debug_invalid([f64::from(x) + 0.5, f64::from(y) + 0.5]);
        assert_eq!(
            near(*got, hatch),
            hatched,
            "`{}` pixel ({x}, {y}): hatched is not {hatched}",
            s.name
        );
    }
}

#[test]
fn structural_scene_spread_hatches_without_an_ensemble() {
    let h = gpu();
    let s = named("s_spread");
    assert_eq!(s.context.grid.e, 0, "no ensemble");
    check_hatched(&s, &render(&h, &s), true);
    let s = named("s_spread_ensemble");
    assert_eq!(s.context.grid.e, 1, "an ensemble");
    check_hatched(&s, &render(&h, &s), false);
}

negative_control!(
    structural_scene_spread_hatches_without_an_ensemble,
    "the spread with an ensemble is not hatched",
    expected = "hatched is not true",
    {
        let s = named("s_spread_ensemble");
        check_hatched(&s, &render(&gpu(), &s), true);
    }
);

/// Checks the tint and hatch scene `name`: a quad with `ancestor_gap > 0` is its state's colour tinted 0.4 of the way to the
/// pink, the rest untinted; a pending quad's pixels on the hatch's lines are the blue, the rest of it its colour beneath;
/// and no pixel is either of `debug_invalid`'s colours.
fn check_overlay(h: &GpuHarness, name: &str, tint: bool, pending: bool) {
    let s = named(name);
    let image = render(h, &s);
    let (width, height) = s.size();
    let hatch = present::debug_invalid([0.5, 0.5]);
    let other = present::debug_invalid([4.5, 0.5]);
    let blue = present::srgb8(mirror::PENDING_SRGB8);
    let (mut tinted, mut lines) = (0, 0);
    for y in 0..height {
        for x in 0..width {
            let q = s.quad(s.context.grid.cell(x, y).quad);
            let frag = [f64::from(x) + 0.5, f64::from(y) + 0.5];
            let mut want = present::dbg_cat(q.state, 5);
            if tint && q.ancestor_gap > 0 {
                want = mirror::mix(
                    want,
                    present::srgb8(mirror::FALLBACK_TINT_SRGB8),
                    mirror::FALLBACK_OPACITY,
                );
                tinted += 1;
            }
            if pending && q.state == 1 && mirror::pending_on(frag) {
                want = blue;
                lines += 1;
            }
            let got = image[(y * width + x) as usize];
            assert!(
                near(got, want),
                "`{name}` pixel ({x}, {y}): {got:?}, not {want:?}"
            );
            assert!(
                !near(got, hatch) && !near(got, other),
                "`{name}` pixel ({x}, {y}) is in debug_invalid's colours"
            );
        }
    }
    assert_eq!(tint, tinted > 0, "`{name}`: tinted pixels {tinted}");
    assert_eq!(pending, lines > 0, "`{name}`: hatch lines {lines}");
}

#[test]
fn structural_scene_overlays_mark_their_quads() {
    let h = gpu();
    check_overlay(&h, "fallback_tint", true, false);
    check_overlay(&h, "pending_hatch", false, true);
}

negative_control!(
    structural_scene_overlays_mark_their_quads,
    "the pending scene checked as tinted fails",
    expected = "`pending_hatch` pixel",
    check_overlay(&gpu(), "pending_hatch", true, true)
);

/// The row of a width window the vertical line crosses away from the horizontal one: 4 px above the corner.
const ROW: u32 = 4;

/// The measured width of the boundary line in scene `name`'s window, in pixels, on its GPU render.
fn measured(h: &GpuHarness, name: &str) -> f64 {
    let s = named(name);
    let image = render(h, &s);
    line_width(
        &image,
        s.size().0,
        ROW,
        present::ramp_grey(MID_GREY_L),
        [1.0; 3],
    )
}

/// Checks that the four width scenes measure one width, `want` px, and render one window, and that the thresholded
/// control's two quad sizes measure different widths.
fn check_widths(h: &GpuHarness, cases: [&str; 4], want: f64, control: [&str; 2]) {
    let first = render(h, &named(cases[0]));
    for name in cases {
        let w = measured(h, name);
        eprintln!("{name}: boundary width {w:.4} px");
        assert!(
            (w - want).abs() < 1e-4,
            "`{name}`'s boundary is {w} px wide, not {want}"
        );
        let s = named(name);
        check_image(&s, &render(h, &s), &first, &format!("`{}`'s", cases[0]));
    }
    let [a, b] = control.map(|name| measured(h, name));
    eprintln!("{}: {a:.4} px; {}: {b:.4} px", control[0], control[1]);
    assert!(
        (a - b).abs() > 0.5,
        "the thresholded control measures {a} and {b} px: one width at both quad sizes"
    );
}

#[test]
fn structural_scene_boundary_width_is_constant() {
    let h = gpu();
    check_widths(
        &h,
        [
            "width_d3_q16",
            "width_d3_q64",
            "width_d20_q16",
            "width_d20_q64",
        ],
        2.0,
        ["threshold_q16", "threshold_q64"],
    );
}

negative_control!(
    structural_scene_boundary_width_is_constant,
    "the thresholded control in place of the overlay measures different widths",
    expected = "px wide, not 2",
    check_widths(
        &gpu(),
        [
            "threshold_q16",
            "threshold_q64",
            "width_d20_q16",
            "width_d20_q64"
        ],
        2.0,
        ["threshold_q16", "threshold_q64"],
    )
);

#[test]
fn structural_scene_unknown_is_refused() {
    match scene("no_such_scene") {
        Ok(_) => panic!("an unknown scene loaded"),
        Err(e) => assert!(
            e.contains("no structural scene `no_such_scene`; the scenes are s_depth"),
            "{e}"
        ),
    }
}

negative_control!(
    structural_scene_unknown_is_refused,
    "a listed scene loads",
    expected = "an unknown scene loaded",
    match scene("s_depth") {
        Ok(_) => panic!("an unknown scene loaded"),
        Err(e) => panic!("{e}"),
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
        .render(h.device(), h.queue())
        .unwrap_or_else(|e| panic!("{e}"));
    let (w, hgt) = s.size();
    assert!(
        out.stdout == encode_image(w, hgt, &pixels),
        "the binary's `{name}` is not `{against}`'s render"
    );
}

#[test]
fn structural_scene_binary_writes_the_render() {
    check_binary("width_d20_q64", "width_d20_q64");
    check_binary("overlays", "overlays");
}

negative_control!(
    structural_scene_binary_writes_the_render,
    "the binary's quad boundaries are not the tile boundaries' render",
    expected = "is not `tile_boundaries`'s render",
    check_binary("quad_boundaries", "tile_boundaries")
);
