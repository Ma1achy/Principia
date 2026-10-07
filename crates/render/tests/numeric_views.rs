//! The numeric field views and the field ramps, rendered through the golden harness's scenes
//! (`validation::golden_scene`; TASK-M1-09):
//! - REQ-RENDER-023: a sample with state = failed and a stored 0.0 renders the ramp colour of 0.0, not the invalid
//!   colour (`debug_fields_raw_*`);
//! - REQ-GEN-012: a diffusion with n < 2 reads NaN and renders the hatch, distinct from a valid slope on the ramp; the
//!   stored sentinels `dmin_pair` 3 and `length` 127 round-trip bit-exact and render as their literal values; `d_min`'s
//!   unset +∞ renders in the "not yet" grey (`diffusion_invalid_fit_*`);
//! - REQ-TOOL-023: `d_min`, `dE_max` and `dLz_max` pack and unpack within f16 eps, and their views render their CPU
//!   twins, the unset `d_min` in the grey (`f16_scalar_views_*`);
//! - REQ-COL-001: the field ramps' invalid lane, the pattern by default and the override colour changing only the
//!   invalid pixels (`field_ramp_*`);
//! - every golden scene renders its CPU twin, far enough from every rounding tie that one reference holds on every
//!   backend (`golden_scenes_*`).
//!
//! Each test registers its negative control (R-176).

use render::present::Rgb;
use validation::golden_scene::{
    self, f16_bits_to_f32, f32_to_f16_bits, fgw_length_raw, scene, Colouring, Look, Scene,
    SimState, STATE_SIM_FAILED,
};
use validation::gpu::GpuHarness;
use validation::negative_control;

/// The largest per-channel difference, linear RGB, between a render and its CPU twin: the renders sit within 6e-7 of
/// their twins on Metal, so 1e-5 leaves a margin and still parts every two looks a test tells apart.
const TOL: f64 = 1e-5;

/// The least distance, in 8-bit steps, of any golden scene's twin from a rounding tie ([`golden_scene::tie_margin`]):
/// some sixty times the 1.5e-4 steps (6e-7 × 255) the renders sit from their twins.
const MIN_TIE_MARGIN: f64 = 0.01;

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

fn named(name: &str) -> Scene {
    scene(name).unwrap_or_else(|e| panic!("{e}"))
}

fn render(h: &GpuHarness, s: &Scene) -> Vec<Rgb> {
    s.render(h.device(), h.queue())
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .map(|p| [p[0], p[1], p[2]].map(f64::from))
        .collect()
}

fn look(s: &Scene, i: u32) -> Look {
    s.look(i).unwrap_or_else(|e| panic!("{e}"))
}

fn near(a: Rgb, b: Rgb) -> bool {
    a.iter().zip(&b).all(|(x, y)| (x - y).abs() <= TOL)
}

/// Checks that every pixel of sample `i` of `image` is `want`'s colour there.
fn check_sample(s: &Scene, image: &[Rgb], i: u32, want: Look, what: &str) {
    let width = s.size().0;
    for (x, y) in s.pixels(i) {
        let got = image[(y * width + x) as usize];
        let colour = want.colour([x, y]);
        assert!(
            near(got, colour),
            "pixel ({x}, {y}) of sample {i} of `{}` is {got:?}, not {what} {colour:?}",
            s.name
        );
    }
}

/// Checks that every pixel of `image` is its CPU twin's ([`Scene::expected`]).
fn check_twin(s: &Scene, image: &[Rgb]) {
    let want = s.expected().unwrap_or_else(|e| panic!("{e}"));
    let width = s.size().0 as usize;
    assert_eq!(image.len(), want.len(), "`{}`: the render's size", s.name);
    for (k, (got, want)) in image.iter().zip(&want).enumerate() {
        assert!(
            near(*got, *want),
            "pixel ({}, {}) of `{}` is {got:?}, which differs from its CPU twin {want:?}",
            k % width,
            k / width,
            s.name
        );
    }
}

/// `image` with sample `i`'s pixels drawn as `look`: the controls' wrong renders.
fn painted(s: &Scene, image: &[Rgb], i: u32, look: Look) -> Vec<Rgb> {
    let width = s.size().0;
    let mut out = image.to_vec();
    for (x, y) in s.pixels(i) {
        out[(y * width + x) as usize] = look.colour([x, y]);
    }
    out
}

/// The ramp's look at the bottom of the measured range: the colour 0.0 takes when it is the scene's least value.
const RAMP_AT_ZERO: Look = Look::Ramp {
    twilight: false,
    t: 0.0,
};

/// The CPU read of sample `i` of `s`, through the kernel's read side.
fn read(s: &Scene, i: u32) -> SimState {
    s.read(i)
}

/// Checks that sample 0 of the `dE_max` or `dLz_max` scene is failed, stores 0.0, and renders the ramp colour of 0.0.
fn check_failed_zero(s: &Scene, image: &[Rgb]) {
    let r = read(s, 0);
    assert_eq!(
        r.state, STATE_SIM_FAILED,
        "`{}`: sample 0 is failed",
        s.name
    );
    let (v, _) = s.value(0);
    assert_eq!(v.to_bits(), 0, "`{}`: sample 0 stores 0.0", s.name);
    assert_eq!(look(s, 0), RAMP_AT_ZERO, "`{}`: 0.0's look", s.name);
    check_sample(s, image, 0, RAMP_AT_ZERO, "the ramp colour of 0.0");
}

#[test]
fn debug_fields_raw_failed_state_zero_renders_the_ramp_colour_of_zero() {
    let h = gpu();
    for name in ["de_max_failed", "dlz_max_failed"] {
        let s = named(name);
        check_failed_zero(&s, &render(&h, &s));
    }
}

negative_control!(
    debug_fields_raw_failed_state_zero_renders_the_ramp_colour_of_zero,
    "a view masking the failed sample, hatching its 0.0, fails the check",
    expected = "not the ramp colour of 0.0",
    {
        let s = named("de_max_failed");
        let image = render(&gpu(), &s);
        check_failed_zero(&s, &painted(&s, &image, 0, Look::Invalid));
    }
);

/// Checks that sample 3 of the `d_min` scene, failed with a stored 0.0, renders on the ramp at 0.0's place: the debug
/// view shows the stored value literally, masking nothing but NaN.
fn check_d_min_failed_zero(s: &Scene, image: &[Rgb]) {
    assert_eq!(read(s, 3).state, STATE_SIM_FAILED, "sample 3 is failed");
    assert_eq!(s.value(3).0.to_bits(), 0, "sample 3 stores 0.0");
    let shown = look(s, 3);
    assert!(
        matches!(
            shown,
            Look::Ramp {
                twilight: false,
                ..
            }
        ),
        "`d_min`'s 0.0 is not on the ramp: {shown:?}"
    );
    check_sample(s, image, 3, shown, "the ramp colour of 0.0");
}

#[test]
fn debug_fields_raw_d_min_failed_state_zero_is_on_the_ramp() {
    let s = named("d_min");
    check_d_min_failed_zero(&s, &render(&gpu(), &s));
}

negative_control!(
    debug_fields_raw_d_min_failed_state_zero_is_on_the_ramp,
    "a d_min view hatching the failed sample's 0.0 fails the check",
    expected = "not the ramp colour of 0.0",
    {
        let s = named("d_min");
        let image = render(&gpu(), &s);
        check_d_min_failed_zero(&s, &painted(&s, &image, 3, Look::Invalid));
    }
);

/// Checks the diffusion scene: n = 0 and n = 1 read NaN and render the hatch; n = 2, the first valid fit, renders on
/// the ramp; the two are distinct.
fn check_diffusion(s: &Scene, image: &[Rgb]) {
    for i in [0, 1] {
        let r = read(s, i);
        assert!(!r.diffusion_slope_valid, "sample {i}: n < 2 is invalid");
        assert_eq!(r.diffusion.to_bits(), 0x7fc0_0000, "sample {i}: reads NaN");
        assert_eq!(look(s, i), Look::Invalid, "sample {i}'s look");
        check_sample(s, image, i, Look::Invalid, "the hatch");
    }
    assert!(read(s, 2).diffusion_slope_valid, "sample 2: n = 2 is valid");
    let valid = look(s, 2);
    assert!(
        matches!(valid, Look::Ramp { .. }),
        "n = 2's look is {valid:?}"
    );
    check_sample(s, image, 2, valid, "a valid slope's ramp colour");
    for (x, y) in s.pixels(2) {
        assert!(
            !near(valid.colour([x, y]), Look::Invalid.colour([x, y])),
            "a valid slope's colour at ({x}, {y}) is the hatch's"
        );
    }
}

#[test]
fn diffusion_invalid_fit_reads_nan_and_renders_the_hatch() {
    let s = named("diffusion");
    check_diffusion(&s, &render(&gpu(), &s));
}

negative_control!(
    diffusion_invalid_fit_reads_nan_and_renders_the_hatch,
    "a view drawing n = 1 on the ramp, as a valid slope, fails the check",
    expected = "not the hatch",
    {
        let s = named("diffusion");
        let image = render(&gpu(), &s);
        let wrong = Look::Ramp {
            twilight: false,
            t: 0.5,
        };
        check_diffusion(&s, &painted(&s, &image, 1, wrong));
    }
);

/// A one-row scene of `dmin_pair`'s view, its samples holding the sentinel 3 and the pairs 0, 1 and 2.
fn dmin_pair_scene() -> Scene {
    let mut s = named("length_view");
    for (i, pair) in [3u32, 0, 1, 2, 3, 0, 1, 2].into_iter().enumerate() {
        s.set.sample(i as u32).word([0; 4], 0).dmin_pair(pair);
    }
    s.colouring = Colouring::View("dmin_pair");
    s
}

/// Checks that `dmin_pair`'s 3 and the word `length`'s 127 round-trip bit-exact through the payload and render as
/// their literal values on the ramp, `dbg_sentinel(3)` and `dbg_sentinel(127)` (R-136).
fn check_sentinels(pair: (&Scene, &[Rgb]), length: (&Scene, &[Rgb])) {
    let (s, image) = pair;
    assert_eq!(read(s, 0).dmin_pair, 3, "`dmin_pair`'s 3 round-trips");
    assert_eq!(s.value(0).0.to_bits(), 3.0f32.to_bits());
    check_sample(s, image, 0, Look::Literal(3.0), "the literal 3");
    let (s, image) = length;
    assert_eq!(
        fgw_length_raw(s.set.word(7)),
        127,
        "the word `length`'s 127 round-trips"
    );
    assert_eq!(look(s, 7), Look::Literal(127.0), "127's look");
    check_sample(s, image, 7, Look::Literal(127.0), "the literal 127");
}

#[test]
fn diffusion_invalid_fit_sentinels_round_trip_and_render_literally() {
    let h = gpu();
    let pair = dmin_pair_scene();
    let length = named("length_view");
    let (a, b) = (render(&h, &pair), render(&h, &length));
    check_sentinels((&pair, &a), (&length, &b));
}

negative_control!(
    diffusion_invalid_fit_sentinels_round_trip_and_render_literally,
    "a length view hatching its sentinel, as an invalid value, fails the check",
    expected = "not the literal 127",
    {
        let h = gpu();
        let pair = dmin_pair_scene();
        let length = named("length_view");
        let (a, b) = (render(&h, &pair), render(&h, &length));
        let b = painted(&length, &b, 7, Look::Invalid);
        check_sentinels((&pair, &a), (&length, &b));
    }
);

/// Checks that samples 0 and 1 of a `d_min` scene, a forced failure and an unstepped sample, hold the unset f16 +∞ by
/// its bits and render the "not yet" grey.
fn check_unset_grey(s: &Scene, image: &[Rgb]) {
    for i in [0, 1] {
        let packed = s.set.simstate(i).packed_a;
        assert_eq!(packed >> 16, 0x7c00, "sample {i} holds f16 +inf");
        assert_eq!(look(s, i), Look::NotYet, "sample {i}'s look");
        check_sample(s, image, i, Look::NotYet, "the \"not yet\" grey");
    }
}

#[test]
fn diffusion_invalid_fit_d_min_unset_renders_the_grey() {
    let s = named("d_min");
    check_unset_grey(&s, &render(&gpu(), &s));
}

negative_control!(
    diffusion_invalid_fit_d_min_unset_renders_the_grey,
    "a view hatching the unset d_min, as if it were NaN, fails the check",
    expected = "not the \"not yet\" grey",
    {
        let s = named("d_min");
        let image = render(&gpu(), &s);
        check_unset_grey(&s, &painted(&s, &image, 1, Look::Invalid));
    }
);

/// f16's machine epsilon, 2⁻¹⁰.
const F16_EPS: f64 = 1.0 / 1024.0;

/// Checks that each of `values` packs into f16 and unpacks within f16 eps of itself, relative, through `round`.
fn check_f16(values: &[f32], round: impl FnMut(f32) -> f32) {
    let mut round = round;
    for &x in values {
        let y = round(x);
        let err = (f64::from(y) - f64::from(x)).abs();
        assert!(
            err <= F16_EPS * f64::from(x).abs(),
            "{x} unpacks as {y}, not within f16 eps"
        );
    }
}

/// The values the f16 check packs: the scenes', both ends of f16's normal range, and a sweep between.
fn f16_values() -> Vec<f32> {
    let mut v = vec![
        6.103_515_6e-5,
        65504.0,
        2.5e-3,
        0.07,
        0.6,
        1.7,
        4.5e-4,
        90.0,
    ];
    v.extend((0..60).map(|k| 2f32.powf(-14.0 + k as f32 * 0.5) * 1.3));
    v
}

#[test]
fn f16_scalar_views_pack_and_unpack_within_f16_eps() {
    let mut s = named("d_min");
    check_f16(&f16_values(), |x| {
        s.set.sample(4).d_min(x).drift_max(x, x);
        let r = read(&s, 4);
        assert_eq!(r.dE_max.to_bits(), r.dLz_max.to_bits(), "{x}: the drifts");
        assert_eq!(r.d_min.to_bits(), r.dE_max.to_bits(), "{x}: d_min");
        r.d_min
    });
    assert_eq!(
        f16_bits_to_f32(f32_to_f16_bits(f32::INFINITY)),
        f32::INFINITY,
        "the unset value round-trips"
    );
}

negative_control!(
    f16_scalar_views_pack_and_unpack_within_f16_eps,
    "a packing that keeps only seven significand bits, bf16's, is not within f16 eps",
    expected = "not within f16 eps",
    check_f16(&f16_values(), |x| f32::from_bits(x.to_bits() & 0xffff_0000))
);

#[test]
fn f16_scalar_views_render_their_twins() {
    let h = gpu();
    for name in ["d_min", "de_max_failed", "dlz_max_failed"] {
        let s = named(name);
        let image = render(&h, &s);
        check_twin(&s, &image);
        if name == "d_min" {
            check_unset_grey(&s, &image);
            check_sample(&s, &image, 2, Look::Invalid, "the hatch");
        }
    }
}

negative_control!(
    f16_scalar_views_render_their_twins,
    "a d_min view drawing its NaN on the ramp, as a value, differs from its twin",
    expected = "differs from its CPU twin",
    {
        let s = named("d_min");
        let image = render(&gpu(), &s);
        check_twin(&s, &painted(&s, &image, 2, RAMP_AT_ZERO));
    }
);

/// Checks that the field ramp `base` and its override `over` differ exactly at the samples `base` shows invalid,
/// which `over` draws in the override colour.
fn check_override(base: (&Scene, &[Rgb]), over: (&Scene, &[Rgb])) {
    let (s, a) = base;
    let (o, b) = over;
    let width = s.size().0;
    let mut invalid = 0;
    let mut want = a.to_vec();
    for i in 0..s.context.grid.sample_count() {
        let changed = s.pixels(i).into_iter().any(|(x, y)| {
            let k = (y * width + x) as usize;
            !near(a[k], b[k])
        });
        let is_invalid = look(s, i) == Look::Invalid;
        assert_eq!(
            changed, is_invalid,
            "`{}` sample {i}: the override changed it: {changed}, but it is invalid: {is_invalid}",
            s.name
        );
        if is_invalid {
            invalid += 1;
            assert_eq!(look(o, i), Look::Override);
            check_sample(o, b, i, Look::Override, "the override colour");
            want = painted(s, &want, i, Look::Override);
        }
    }
    assert!(invalid > 0, "`{}` has an invalid sample", s.name);
    let same = want.iter().zip(b).all(|(w, g)| near(*w, *g));
    assert!(
        same,
        "`{}` is not `{}` with its invalid samples overridden",
        o.name, s.name
    );
}

#[test]
fn field_ramp_override_changes_only_the_invalid_pixels() {
    let h = gpu();
    for (base, over) in [
        ("length_ramp", "length_ramp_override"),
        ("d_min_ramp", "d_min_ramp_override"),
    ] {
        let (s, o) = (named(base), named(over));
        let (a, b) = (render(&h, &s), render(&h, &o));
        check_override((&s, &a), (&o, &b));
    }
}

negative_control!(
    field_ramp_override_changes_only_the_invalid_pixels,
    "an override that also recolours a valid sample fails the check",
    expected = "the override changed it: true, but it is invalid: false",
    {
        let h = gpu();
        let (s, o) = (named("length_ramp"), named("length_ramp_override"));
        let (a, b) = (render(&h, &s), render(&h, &o));
        let b = painted(&o, &b, 2, Look::Override);
        check_override((&s, &a), (&o, &b));
    }
);

/// Checks the ramps' own legs: `length`'s 127 is invalid on its ramp and literal in its view; `d_min`'s NaN is
/// invalid and its unset value grey, on the ramp as in the view.
fn check_ramp_legs(length: (&Scene, &[Rgb]), d_min: (&Scene, &[Rgb])) {
    let (s, image) = length;
    assert_eq!(look(s, 7), Look::Invalid, "127 on `length`'s ramp");
    check_sample(s, image, 7, Look::Invalid, "the invalid pattern");
    assert_eq!(look(&named("length_view"), 7), Look::Literal(127.0));
    let (s, image) = d_min;
    check_sample(s, image, 2, Look::Invalid, "the invalid pattern");
    check_unset_grey(s, image);
}

#[test]
fn field_ramp_sentinel_and_nan_are_invalid_and_unset_grey() {
    let h = gpu();
    let (l, d) = (named("length_ramp"), named("d_min_ramp"));
    let (a, b) = (render(&h, &l), render(&h, &d));
    check_ramp_legs((&l, &a), (&d, &b));
}

negative_control!(
    field_ramp_sentinel_and_nan_are_invalid_and_unset_grey,
    "a length ramp drawing 127 on the ramp, as the view does, fails the check",
    expected = "not the invalid pattern",
    {
        let h = gpu();
        let (l, d) = (named("length_ramp"), named("d_min_ramp"));
        let (a, b) = (render(&h, &l), render(&h, &d));
        let a = painted(&l, &a, 7, Look::Literal(127.0));
        check_ramp_legs((&l, &a), (&d, &b));
    }
);

#[test]
fn golden_scenes_render_their_twins() {
    let h = gpu();
    for name in golden_scene::NAMES {
        let s = named(name);
        check_twin(&s, &render(&h, &s));
    }
}

negative_control!(
    golden_scenes_render_their_twins,
    "an ftle view hatching a valid sample differs from its twin",
    expected = "differs from its CPU twin",
    {
        let s = named("ftle");
        let image = render(&gpu(), &s);
        check_twin(&s, &painted(&s, &image, 4, Look::Invalid));
    }
);

/// Checks that `pixels` sit at least [`MIN_TIE_MARGIN`] from every rounding tie.
fn check_tie_free(name: &str, pixels: &[Rgb]) {
    let margin = golden_scene::tie_margin(pixels);
    assert!(
        margin >= MIN_TIE_MARGIN,
        "`{name}` sits {margin} 8-bit steps from a rounding tie, nearer than {MIN_TIE_MARGIN}"
    );
}

#[test]
fn golden_scenes_are_tie_free() {
    for name in golden_scene::NAMES {
        let s = named(name);
        check_tie_free(name, &s.expected().unwrap_or_else(|e| panic!("{e}")));
    }
}

negative_control!(
    golden_scenes_are_tie_free,
    "a pixel at 127.5 / 255 sits on a tie",
    expected = "from a rounding tie",
    check_tie_free("a tie", &[[0.0, 0.0, 127.5 / 255.0]])
);
