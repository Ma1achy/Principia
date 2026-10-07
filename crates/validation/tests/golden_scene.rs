//! The golden harness's synthetic scenes (`validation::golden_scene`; RQ-229; TASK-M1-09) and their binary,
//! `golden_harness`: each scene holds the samples its case documents, reads each field through the kernel's read side,
//! sets its node's params, renders its CPU twin on the GPU, and stays clear of every rounding tie; the binary lists the
//! scenes and writes a scene's render in the float image's format. The renders' meaning, per acceptance line, is
//! `render/tests/numeric_views.rs`'s.
//!
//! Each test registers its negative control (R-176).

use std::process::Command;

use render::present::Rgb;
use validation::golden_scene::{self, encode_image, scene, tie_margin, Colouring, Look, Scene};
use validation::gpu::GpuHarness;
use validation::negative_control;

fn named(name: &str) -> Scene {
    scene(name).unwrap_or_else(|e| panic!("{e}"))
}

/// A look's kind: `I`nvalid, `L`iteral, `N`ot yet, `R`amp, `O`verride.
fn kind(l: Look) -> char {
    match l {
        Look::Invalid => 'I',
        Look::Literal(_) => 'L',
        Look::NotYet => 'N',
        Look::Ramp { .. } => 'R',
        Look::Override => 'O',
    }
}

/// The kinds of `s`'s eight samples.
fn kinds(s: &Scene) -> String {
    (0..8)
        .map(|i| kind(s.look(i).unwrap_or_else(|e| panic!("{e}"))))
        .collect()
}

/// Each scene: its field and its samples' kinds.
const SCENES: [(&str, &str, &str); 10] = [
    ("ftle", "ftle", "IIRRRRRR"),
    ("diffusion", "diffusion", "IIRRRRRR"),
    ("de_max_failed", "dE_max", "RRRRRRRR"),
    ("dlz_max_failed", "dLz_max", "RRRRRRRR"),
    ("d_min", "d_min", "NNIRRRRR"),
    ("length_view", "length", "RRRRRRRL"),
    ("length_ramp", "length", "RRRRRRRI"),
    ("length_ramp_override", "length", "RRRRRRRO"),
    ("d_min_ramp", "d_min", "NNIRRRRR"),
    ("d_min_ramp_override", "d_min", "NNORRRRR"),
];

/// Checks that scene `name` colours `field` and shows `want`'s kinds.
fn check_scene(name: &str, field: &str, want: &str) {
    let s = named(name);
    assert_eq!(s.name, name);
    assert_eq!(s.field(), field, "`{name}`'s field");
    assert_eq!(
        kinds(&s),
        want,
        "`{name}`'s samples show {}, not {want}",
        kinds(&s)
    );
}

#[test]
fn golden_scene_scenes_hold_their_samples() {
    assert_eq!(golden_scene::NAMES.len(), SCENES.len());
    for ((name, field, want), listed) in SCENES.iter().zip(golden_scene::NAMES) {
        assert_eq!(*name, listed, "the scenes' order");
        check_scene(name, field, want);
    }
    for name in ["de_max_failed", "dlz_max_failed"] {
        let s = named(name);
        let t = |i| match s.look(i) {
            Ok(Look::Ramp { t, .. }) => t,
            other => panic!("{other:?}"),
        };
        assert_eq!(
            (t(0), t(7)),
            (0.0, 1.0),
            "`{name}`: 0.0 and the largest drift end the range"
        );
    }
}

negative_control!(
    golden_scene_scenes_hold_their_samples,
    "the override scene expected to hatch its NaN, as the pattern scene does, fails the check",
    expected = "`d_min_ramp_override`'s samples show",
    check_scene("d_min_ramp_override", "d_min", "NNIRRRRR")
);

#[test]
fn golden_scene_unknown_is_refused() {
    match scene("no_such_scene") {
        Ok(_) => panic!("an unknown scene loaded"),
        Err(e) => assert!(
            e.contains("no golden scene `no_such_scene`; the scenes are ftle, diffusion"),
            "{e}"
        ),
    }
    let mut s = named("length_view");
    s.colouring = Colouring::View("no_such_field");
    check_colour_refused(&s, "the catalogue has no view of `no_such_field`");
    s.colouring = Colouring::View("dmin_pair");
    assert!(s.colour().is_ok(), "`dmin_pair` has a view");
    match s.look(0) {
        Ok(l) => panic!("`dmin_pair`'s view has a numeric look {l:?}"),
        Err(e) => assert!(e.contains("`dmin_pair` has no numeric view"), "{e}"),
    }
}

/// Checks that `s`'s colour is refused naming `why`.
fn check_colour_refused(s: &Scene, why: &str) {
    match s.colour() {
        Ok(_) => panic!("`{}`'s colour is accepted", s.field()),
        Err(e) => assert!(e.contains(why), "{e}"),
    }
}

negative_control!(
    golden_scene_unknown_is_refused,
    "a catalogued field's view is accepted",
    expected = "`ftle`'s colour is accepted",
    {
        let mut s = named("length_view");
        s.colouring = Colouring::View("ftle");
        check_colour_refused(&s, "the catalogue has no view");
    }
);

/// Checks that scene `name`'s `value` is the kernel read's `want` of each sample.
fn check_value(s: &Scene, want: impl Fn(&kernel::payload::SimState) -> (f32, bool)) {
    for i in 0..8 {
        let (v, gate) = s.value(i);
        let (w, g) = want(&s.read(i));
        assert_eq!(
            (v.to_bits(), gate),
            (w.to_bits(), g),
            "`{}` sample {i}: the value is not the field's read",
            s.name
        );
    }
}

#[test]
fn golden_scene_value_reads_the_field() {
    check_value(&named("ftle"), |r| (r.ftle, r.ftle_valid));
    check_value(&named("diffusion"), |r| {
        (r.diffusion, r.diffusion_slope_valid)
    });
    check_value(&named("de_max_failed"), |r| (r.dE_max, true));
    check_value(&named("dlz_max_failed"), |r| (r.dLz_max, true));
    check_value(&named("d_min"), |r| (r.d_min, true));
    check_value(&named("length_ramp"), |r| {
        (golden_scene::fgw_length_raw(r.word) as f32, true)
    });
    let mut s = named("length_view");
    for i in 0..8 {
        s.set.sample(i).dmin_pair(i % 4);
    }
    s.colouring = Colouring::View("dmin_pair");
    check_value(&s, |r| (r.dmin_pair as f32, true));
}

negative_control!(
    golden_scene_value_reads_the_field,
    "dLz_max's scene read as dE_max differs",
    expected = "the value is not the field's read",
    check_value(&named("dlz_max_failed"), |r| (r.dE_max, true))
);

/// Checks scene `name`'s params: `want`.
fn check_params(name: &str, want: &[(&str, Vec<f64>)]) {
    let got = named(name).params().unwrap_or_else(|e| panic!("{e}"));
    let got: Vec<(&str, Vec<f64>)> = got.iter().map(|(n, v)| (n.as_str(), v.clone())).collect();
    assert_eq!(got, want, "`{name}`'s params");
}

#[test]
fn golden_scene_params_are_the_nodes() {
    let s = named("ftle");
    let values: Vec<f32> = (0..8).map(|i| s.value(i).0).collect();
    assert_eq!(
        s.params().unwrap_or_else(|e| panic!("{e}")),
        vec![("u_range".to_owned(), vec![0.15f32.into(), 3.05f32.into()])],
        "ftle's measured range, over {values:?}"
    );
    check_params("length_ramp", &[("INVALID_OVERRIDE", vec![0.0])]);
    check_params("length_ramp_override", &[("INVALID_OVERRIDE", vec![1.0])]);
    let mut pair = named("length_view");
    pair.colouring = Colouring::View("dmin_pair");
    assert!(pair.params().unwrap_or_else(|e| panic!("{e}")).is_empty());
}

negative_control!(
    golden_scene_params_are_the_nodes,
    "the override scene's params are not the pattern's",
    expected = "`length_ramp_override`'s params",
    check_params("length_ramp_override", &[("INVALID_OVERRIDE", vec![0.0])])
);

#[test]
fn golden_scene_pixels_are_the_samples_tile() {
    let s = named("ftle");
    assert_eq!(s.size(), (64, 8));
    for i in 0..8 {
        check_tile(&s, i, 8 * i);
    }
}

/// Checks that sample `i`'s pixels are the 8 × 8 tile from column `x0`.
fn check_tile(s: &Scene, i: u32, x0: u32) {
    let px = s.pixels(i);
    let want: Vec<(u32, u32)> = (0..8)
        .flat_map(|y| (x0..x0 + 8).map(move |x| (x, y)))
        .collect();
    assert_eq!(px, want, "sample {i}'s pixels are not its tile");
}

negative_control!(
    golden_scene_pixels_are_the_samples_tile,
    "sample 1's pixels are not sample 0's tile",
    expected = "sample 1's pixels are not its tile",
    check_tile(&named("ftle"), 1, 0)
);

/// The largest per-channel difference between a render and its twin the check allows.
const TOL: f64 = 1e-5;

/// Checks `image` against `s`'s twin.
fn check_twin(s: &Scene, image: &[[f32; 4]]) {
    let want = s.expected().unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(image.len(), want.len());
    for (k, (got, want)) in image.iter().zip(&want).enumerate() {
        let near = (0..3).all(|c| (f64::from(got[c]) - want[c]).abs() <= TOL) && got[3] == 1.0;
        assert!(
            near,
            "`{}` pixel {k}: {got:?} differs from its CPU twin {want:?}",
            s.name
        );
    }
}

#[test]
fn golden_scene_renders_its_twin() {
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    for name in golden_scene::NAMES {
        let s = named(name);
        let image = s
            .render(h.device(), h.queue())
            .unwrap_or_else(|e| panic!("{e}"));
        check_twin(&s, &image);
    }
}

negative_control!(
    golden_scene_renders_its_twin,
    "the override scene's render against the pattern scene's twin differs",
    expected = "differs from its CPU twin",
    {
        let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
        let image = named("length_ramp_override")
            .render(h.device(), h.queue())
            .unwrap_or_else(|e| panic!("{e}"));
        check_twin(&named("length_ramp"), &image);
    }
);

/// Checks that `pixels` sit `want` 8-bit steps from a tie, within 1e-9.
fn check_margin(pixels: &[Rgb], want: f64) {
    let got = tie_margin(pixels);
    assert!(
        got == want || (got - want).abs() < 1e-9,
        "the margin is {got}, not {want}"
    );
}

#[test]
fn golden_scene_tie_margin_measures_the_nearest_tie() {
    check_margin(&[[0.0, 0.0, 0.0]], 0.5);
    check_margin(&[[1.0, 1.0, 1.0]], 0.5);
    check_margin(&[[2.0, -1.0, 0.0]], 0.5);
    check_margin(&[[0.25 / 255.0, 0.0, 1.0]], 0.25);
    check_margin(&[[0.0, 0.0, 0.0], [0.0, 10.4 / 255.0, 0.0]], 0.1);
    check_margin(&[], f64::INFINITY);
}

negative_control!(
    golden_scene_tie_margin_measures_the_nearest_tie,
    "a pixel half a step up sits on a tie",
    expected = "the margin is",
    check_margin(&[[0.5 / 255.0, 0.0, 0.0]], 0.5)
);

/// Checks that `bytes` is `MAGIC`, `width`, `height` and `pixels`.
fn check_image(bytes: &[u8], width: u32, height: u32, pixels: &[[f32; 4]]) {
    let mut want = golden_scene::MAGIC.to_vec();
    want.extend(width.to_le_bytes());
    want.extend(height.to_le_bytes());
    for p in pixels {
        for c in p {
            want.extend(c.to_le_bytes());
        }
    }
    assert!(
        bytes == want.as_slice(),
        "the image's bytes are not the format's"
    );
}

#[test]
fn golden_scene_image_format() {
    let pixels = [[0.25, 0.5, 1.0, 1.0], [-1.0, 2.0, 0.0, 0.5]];
    check_image(&encode_image(2, 1, &pixels), 2, 1, &pixels);
    assert_eq!(encode_image(0, 0, &[]).len(), 16);
}

negative_control!(
    golden_scene_image_format,
    "an image with width and height swapped is not the format's",
    expected = "not the format's",
    {
        let pixels = [[0.25, 0.5, 1.0, 1.0], [-1.0, 2.0, 0.0, 0.5]];
        check_image(&encode_image(1, 2, &pixels), 2, 1, &pixels);
    }
);

fn harness(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_golden_harness"))
        .args(args)
        .output()
        .expect("run golden_harness")
}

/// Checks that `golden_harness --scene name` writes the scene's render in the image format.
fn check_binary(name: &str, against: &str) {
    let out = harness(&["--scene", name]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let s = named(against);
    let pixels = s
        .render(h.device(), h.queue())
        .unwrap_or_else(|e| panic!("{e}"));
    let (w, hgt) = s.size();
    check_image(&out.stdout, w, hgt, &pixels);
}

#[test]
fn golden_scene_binary_writes_the_render() {
    check_binary("length_view", "length_view");
    let list = harness(&["--list"]);
    assert!(list.status.success());
    let names: Vec<String> = String::from_utf8_lossy(&list.stdout)
        .lines()
        .map(str::to_owned)
        .collect();
    assert_eq!(names, golden_scene::NAMES, "--list prints the scenes");
    for (args, why) in [
        (
            &["--scene", "no_such_scene"][..],
            "no golden scene `no_such_scene`",
        ),
        (&["--scene"][..], "unrecognised arguments `--scene`"),
        (&[][..], "usage: golden_harness (--scene <name> | --list)"),
    ] {
        let out = harness(args);
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success(), "{args:?} succeeded");
        assert!(
            err.contains("golden_harness: ") && err.contains(why),
            "{args:?}: {err}"
        );
    }
}

negative_control!(
    golden_scene_binary_writes_the_render,
    "the binary's ramp scene is not the view scene's render",
    expected = "not the format's",
    check_binary("length_ramp", "length_view")
);
