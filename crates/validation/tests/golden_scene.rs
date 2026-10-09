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
    let view = matches!(s.colouring, Colouring::View(_));
    assert_eq!(
        view,
        !name.contains("ramp"),
        "`{name}` is coloured by a view: {view}"
    );
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
        let (v, gate) = s.value(i).unwrap_or_else(|e| panic!("{e}"));
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
    let values: Vec<f32> = (0..8)
        .map(|i| s.value(i).unwrap_or_else(|e| panic!("{e}")).0)
        .collect();
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

/// Checks that scene `name`'s samples `from…` read `want`, each within `rel` of it, relative.
fn check_values(name: &str, from: u32, want: &[f32], rel: f32) {
    let s = named(name);
    for (k, &w) in want.iter().enumerate() {
        let i = from + k as u32;
        let v = s.value(i).unwrap_or_else(|e| panic!("{e}")).0;
        assert!(
            (v - w).abs() <= rel * w.abs(),
            "`{name}` sample {i} reads {v}, not {w}"
        );
    }
}

/// `x` rounded to f16, as the payload stores it.
fn f16(x: f32) -> f32 {
    golden_scene::f16_bits_to_f32(golden_scene::f32_to_f16_bits(x))
}

/// The scenes' documented values: `ftle` is `S` (n = 100, dt = 0.01); `diffusion` at n = 2 is 0.6 and the next slope is
/// negative; the f16 drifts and `d_min` are their values rounded to f16, `dLz_max` 0.6 of `dE_max`'s; the word lengths.
#[test]
fn golden_scene_values_are_the_scenes() {
    check_values("ftle", 2, &[0.15, 0.4, 0.85, 1.3, 2.1, 3.05], 1e-5);
    check_values("diffusion", 2, &[0.6], 1e-5);
    let slope = named("diffusion")
        .value(3)
        .unwrap_or_else(|e| panic!("{e}"))
        .0;
    assert!(
        slope < 0.0,
        "`diffusion` sample 3 is a negative slope: {slope}"
    );
    let drifts = [3e-6f32, 4.5e-4, 7e-3, 0.06, 0.55, 4.0, 90.0];
    check_values("de_max_failed", 1, &drifts.map(f16), 0.0);
    check_values("dlz_max_failed", 1, &drifts.map(|v| f16(v * 0.6)), 0.0);
    check_values("d_min", 4, &[2.5e-3f32, 0.07, 0.6, 1.7].map(f16), 0.0);
    check_values(
        "length_view",
        0,
        &[0.0, 9.0, 23.0, 38.0, 51.0, 64.0, 76.0, 127.0],
        0.0,
    );
}

negative_control!(
    golden_scene_values_are_the_scenes,
    "a positive slope where the scene has n = 2's 0.6 is not the scene's",
    expected = "reads",
    check_values("diffusion", 2, &[-0.6], 1e-5)
);

/// Checks that `d_min`'s NaN sample, 2, holds the f16 quiet NaN in its high half and keeps the low half of its word
/// (its state, detail and pair) as the valid samples have it.
fn check_nan_word(s: &Scene) {
    let w = s.set.simstate(2).packed_a;
    let valid = s.set.simstate(4).packed_a;
    assert_eq!(
        w >> 16,
        0x7e00,
        "`{}` sample 2's d_min bits: {w:#010x}",
        s.name
    );
    assert_eq!(
        w & 0xffff,
        valid & 0xffff,
        "`{}` sample 2's low half is not a valid sample's: {w:#010x}",
        s.name
    );
}

#[test]
fn golden_scene_d_min_nan_keeps_its_word() {
    check_nan_word(&named("d_min"));
}

negative_control!(
    golden_scene_d_min_nan_keeps_its_word,
    "a NaN sample whose low half is all ones is not the scene's",
    expected = "low half is not a valid sample's",
    {
        let mut s = named("d_min");
        let w = s.set.simstate(2).packed_a;
        s.set.sample(2).packed_a(w | 0xffff);
        check_nan_word(&s);
    }
);

/// Checks that `s`'s twin reads sample 2 with the masses of its `ICDescriptor`: `energy_drift`, the read's one
/// mass-dependent value, is the kernel's for those masses. Sample 2 is given a finite energy first.
fn check_masses(mut s: Scene) {
    s.set
        .sample(2)
        .r([[1.0, 0.0], [-0.5, 0.5], [-0.5, -0.5]])
        .p([[0.0, 0.3], [0.2, -0.1], [-0.2, -0.2]])
        .E_0(-1.0);
    let ic = *s.set.ic(2);
    let c = &s.context;
    let params = kernel::payload::ReadParams {
        dt_macro: c.dt_macro,
        delta_0: c.delta_0,
        n_renorm: c.n_renorm,
        horizon_steps: c.horizon_steps,
    };
    let want = kernel::payload::sim_state_from_ftle(
        s.set.simstate(2),
        s.set.word(2),
        true,
        kernel::payload::canonical_nan(),
        false,
        [ic.m0, ic.m1, ic.m2],
        &params,
    )
    .energy_drift;
    let got = s.read(2).energy_drift;
    assert!(
        want.is_finite(),
        "sample 2's energy drift is finite: {want}"
    );
    assert_eq!(
        got.to_bits(),
        want.to_bits(),
        "`{}`'s twin reads an energy drift of {got}, not the sample's masses' {want}",
        s.name
    );
}

#[test]
fn golden_scene_read_uses_the_sample_masses() {
    check_masses(named("ftle"));
}

negative_control!(
    golden_scene_read_uses_the_sample_masses,
    "a sample whose masses are not the twin's thirds",
    expected = "not the sample's masses'",
    {
        let mut s = named("ftle");
        let ic = s.set.ic(2);
        ic.m0 = 0.5;
        ic.m1 = 0.25;
        ic.m2 = 0.25;
        check_masses(s);
    }
);

/// Checks that `s`, whose field `Scene::value` doesn't list, refuses to read it: its value, its look and its params
/// (whose `u_range` is measured from the values) are each an error naming the field, never another field's reading.
fn check_unlisted(s: &Scene, field: &str) {
    let why = format!("its field `{field}` has no read in `Scene::value`");
    match s.value(0) {
        Ok(v) => panic!("`{}` reads `{field}` as {v:?}", s.name),
        Err(e) => assert!(e.contains(&why), "{e}"),
    }
    match s.look(0) {
        Ok(l) => panic!("`{}` shows `{field}` as {l:?}", s.name),
        Err(e) => assert!(e.contains(&why), "{e}"),
    }
    match s.params() {
        Ok(p) => panic!("`{}` measures `{field}`'s params as {p:?}", s.name),
        Err(e) => assert!(e.contains(&why), "{e}"),
    }
}

#[test]
fn golden_scene_unlisted_field_is_an_error() {
    let mut s = named("ftle");
    s.colouring = Colouring::View("closure_min");
    check_unlisted(&s, "closure_min");
    for name in golden_scene::NAMES {
        let s = named(name);
        assert!(s.value(0).is_ok(), "`{name}` reads its own field");
    }
}

negative_control!(
    golden_scene_unlisted_field_is_an_error,
    "a scene of a listed field, which reads",
    expected = "reads `ftle` as",
    check_unlisted(&named("ftle"), "ftle")
);

/// Checks that `s`, coloured by the `ICDescriptor` member `rho_angle`, reads each sample's own value of it, as set.
fn check_ic_reads(s: &Scene, want: &[f32]) {
    for (i, &w) in want.iter().enumerate() {
        let (v, gate) = s.value(i as u32).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            (v.to_bits(), gate),
            (w.to_bits(), true),
            "`rho_angle` sample {i} reads {v}, not {w}"
        );
    }
}

/// A scene of `rho_angle`, the sample `i`'s angle `0.25 + i`.
fn rho_angle_scene() -> (Scene, Vec<f32>) {
    let mut s = named("ftle");
    s.colouring = Colouring::View("rho_angle");
    let want: Vec<f32> = (0..8).map(|i| 0.25 + i as f32).collect();
    for (i, &w) in want.iter().enumerate() {
        s.set.ic(i as u32).rho_angle = w;
    }
    (s, want)
}

#[test]
fn golden_scene_reads_ic_members() {
    let (s, want) = rho_angle_scene();
    check_ic_reads(&s, &want);
    let params = s.params().unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(params.len(), 1, "a view's one param, `u_range`: {params:?}");
    let mut s = named("ftle");
    s.colouring = Colouring::View("_pad");
    let e = s.value(0).expect_err("`_pad` is no f32 member, so no read");
    assert!(e.contains("its field `_pad` has no read"), "{e}");
}

negative_control!(
    golden_scene_reads_ic_members,
    "angles other than the samples' own",
    expected = "`rho_angle` sample 0 reads 0.25, not 1.25",
    {
        let (s, want) = rho_angle_scene();
        check_ic_reads(&s, &want.iter().map(|w| w + 1.0).collect::<Vec<_>>());
    }
);
