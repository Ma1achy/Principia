//! The numeric field views and the field ramps, rendered through the golden harness's scenes
//! (`validation::golden_scene`; TASK-M1-09):
//! - REQ-RENDER-023: a sample with state = failed and a stored 0.0 renders the ramp colour of 0.0, not the invalid
//!   colour (`debug_fields_raw_*`);
//! - REQ-GEN-012: a diffusion with n < 2 reads NaN and renders the hatch, distinct from a valid slope on the ramp; the
//!   stored sentinels `dmin_pair` 3 and `length` 127 round-trip bit-exact and render as their literal values; `d_min`'s
//!   unset +∞ renders in the "not yet" style, the grey with its dot stipple (R-399; `diffusion_invalid_fit_*`);
//! - REQ-TOOL-023: `d_min`, `dE_max` and `dLz_max` pack and unpack within f16 eps, and their views render their CPU
//!   twins, the unset `d_min` in the "not yet" style (`f16_scalar_views_*`);
//! - REQ-COL-001: the field ramps' invalid lane, the pattern by default and the override colour changing only the
//!   invalid pixels (`field_ramp_*`);
//! - REQ-RENDER-084 (R-400, R-403; TASK-M1-17): `E_0`, `Lz_0` and `C_ty`, whose declared ranges span zero, render on
//!   `ramp_coolwarm`, 0 at the neutral and the two signs on its two sides; `K_0`, `V_0` and `virial_ratio` on
//!   `ramp_viridis`, increasing, `K_0`'s minimum and `V_0`'s most negative value at the dark end, in the views'
//!   default, auto mode (`range_scenes_*`);
//! - every golden scene renders its CPU twin, far enough from every rounding tie that one reference holds on every
//!   backend (`golden_scenes_*`);
//! - REQ-COL-070 (R-436; TASK-M2-31): `rho_ratio` at `+∞` and `V_0` and `E_0` at `−∞` draw the hatch, the image
//!   bit-identical to the one with a NaN in their place, so the measured range left as it was; and `d_min` at its unset
//!   `+∞` draws the "not yet" style, bit-identical to the field ramp's, so the order (the unset line before the
//!   non-finite guard) holds on the GPU (`non_finite_*`).
//!
//! Each test registers its negative control (R-176).

use ledger::gen::numeric::Ramp;
use render::colour::field_ramp::FieldRamp;
use render::present::Rgb;
use validation::golden_scene::{
    self, f16_bits_to_f32, f32_to_f16_bits, fgw_length_raw, row_scene, scene, Colouring, Look,
    Scene, SimState, RANGE_SCENES, STATE_SIM_FAILED,
};
use validation::gpu::GpuHarness;
use validation::negative_control;

/// The largest per-channel difference, linear RGB, between a render and its CPU twin: the renders sit within 6e-7 of
/// their twins on Metal, so 1e-5 leaves a margin and still parts every two looks a test tells apart.
const TOL: f64 = 1e-5;

/// The least distance, in 8-bit steps, of any golden scene's twin from a rounding tie ([`golden_scene::tie_margin`]):
/// some sixty times the 1.5e-4 steps (6e-7 × 255) the renders sit from their twins. A pixel the fragment returns
/// unchanged from a uniform, an outcome swatch, is the same bits on every backend and is not measured
/// (`Scene::tie_checked`; `render/tests/outcome_state.rs` checks its bits).
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
    ramp: Ramp::Viridis,
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
    let (v, _) = s.value(0).unwrap_or_else(|e| panic!("{e}"));
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
    assert_eq!(
        s.value(3).unwrap_or_else(|e| panic!("{e}")).0.to_bits(),
        0,
        "sample 3 stores 0.0"
    );
    let shown = look(s, 3);
    assert!(
        matches!(
            shown,
            Look::Ramp {
                ramp: Ramp::Viridis,
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
            ramp: Ramp::Coolwarm,
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
    assert_eq!(
        s.value(0).unwrap_or_else(|e| panic!("{e}")).0.to_bits(),
        3.0f32.to_bits()
    );
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
/// its bits and render the "not yet" style, pixel by pixel (R-399).
fn check_unset_not_yet(s: &Scene, image: &[Rgb]) {
    for i in [0, 1] {
        let packed = s.set.simstate(i).packed_a;
        assert_eq!(packed >> 16, 0x7c00, "sample {i} holds f16 +inf");
        assert_eq!(look(s, i), Look::NotYet, "sample {i}'s look");
        check_sample(s, image, i, Look::NotYet, "the \"not yet\" style");
    }
}

#[test]
fn diffusion_invalid_fit_d_min_unset_renders_not_yet() {
    let s = named("d_min");
    check_unset_not_yet(&s, &render(&gpu(), &s));
}

negative_control!(
    diffusion_invalid_fit_d_min_unset_renders_not_yet,
    "a view drawing the unset d_min in the flat grey, before R-399, fails the check",
    expected = "not the \"not yet\" style",
    {
        let s = named("d_min");
        let image = render(&gpu(), &s);
        let flat = Look::Flat(render::present::not_yet());
        check_unset_not_yet(&s, &painted(&s, &image, 1, flat));
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
        4.6e-4,
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
            check_unset_not_yet(&s, &image);
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
/// invalid and its unset value in the "not yet" style, on the ramp as in the view.
fn check_ramp_legs(length: (&Scene, &[Rgb]), d_min: (&Scene, &[Rgb])) {
    let (s, image) = length;
    assert_eq!(look(s, 7), Look::Invalid, "127 on `length`'s ramp");
    check_sample(s, image, 7, Look::Invalid, "the invalid pattern");
    assert_eq!(look(&named("length_view"), 7), Look::Literal(127.0));
    let (s, image) = d_min;
    check_sample(s, image, 2, Look::Invalid, "the invalid pattern");
    check_unset_not_yet(s, image);
}

#[test]
fn field_ramp_sentinel_and_nan_are_invalid_and_unset_not_yet() {
    let h = gpu();
    let (l, d) = (named("length_ramp"), named("d_min_ramp"));
    let (a, b) = (render(&h, &l), render(&h, &d));
    check_ramp_legs((&l, &a), (&d, &b));
}

negative_control!(
    field_ramp_sentinel_and_nan_are_invalid_and_unset_not_yet,
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
        check_tie_free(name, &s.tie_checked().unwrap_or_else(|e| panic!("{e}")));
    }
}

negative_control!(
    golden_scenes_are_tie_free,
    "a pixel at 127.5 / 255 sits on a tie",
    expected = "from a rounding tie",
    check_tie_free("a tie", &[[0.0, 0.0, 127.5 / 255.0]])
);

/// Checks a scene of [`RANGE_SCENES`], `values` its field's samples, against its render `image`: a field whose
/// declared range spans zero on `ramp_coolwarm`, 0 at `t = ½`, the neutral, a negative value below and a positive
/// above; any other on `ramp_viridis`, `t` increasing in the value, its least value at `t = 0`, the dark end; every
/// sample its look's colour.
fn check_range_scene(s: &Scene, values: &[f32; 8], image: &[Rgb]) {
    let signed = matches!(s.field(), "E_0" | "Lz_0" | "C_ty");
    let mut placed = Vec::new();
    for (i, &v) in (0u32..).zip(values) {
        let shown = look(s, i);
        let Look::Ramp { ramp, t } = shown else {
            panic!("`{}` sample {i} is {shown:?}, not on a ramp", s.name);
        };
        let want = if signed {
            Ramp::Coolwarm
        } else {
            Ramp::Viridis
        };
        assert_eq!(ramp, want, "`{}` sample {i}'s ramp", s.name);
        if signed {
            let side = if v == 0.0 {
                t == 0.5
            } else if v < 0.0 {
                t < 0.5
            } else {
                t > 0.5
            };
            assert!(
                side,
                "`{}`: {v} is at t = {t}, not on its side of the neutral",
                s.name
            );
        }
        check_sample(s, image, i, shown, "its ramp colour");
        placed.push((v, t));
    }
    placed.sort_by(|a, b| a.0.total_cmp(&b.0));
    assert!(
        placed.windows(2).all(|w| w[0].1 < w[1].1),
        "`{}` does not increase in the value: {placed:?}",
        s.name
    );
    if !signed {
        assert_eq!(placed[0].1, 0.0, "`{}`'s least value is not dark", s.name);
    }
}

#[test]
fn range_scenes_follow_the_declared_range() {
    let h = gpu();
    for (name, _, values) in RANGE_SCENES {
        let s = named(name);
        check_range_scene(&s, &values, &render(&h, &s));
    }
}

negative_control!(
    range_scenes_follow_the_declared_range,
    "E_0's zero drawn mid-viridis, as before R-400, is not the neutral",
    expected = "not its ramp colour",
    {
        let (name, _, values) = RANGE_SCENES[0];
        let s = named(name);
        let image = render(&gpu(), &s);
        let mid = Look::Ramp {
            ramp: Ramp::Viridis,
            t: 0.5,
        };
        check_range_scene(&s, &values, &painted(&s, &image, 4, mid));
    }
);

// ── R-436 (TASK-M2-31): a non-finite value draws the hatch on the GPU (REQ-COL-070) ──────────────────────────────────

/// The sample of a pole scene that holds the pole's value.
const POLE: u32 = 3;

/// The pole scenes: each field's view over a row of eight samples, its finite values, the pole's value at [`POLE`]
/// (the 0.0 there is replaced), and the pole's f32 bits: `rho_ratio`'s `+∞` at `α = 0`, `V_0`'s and `E_0`'s `−∞` at the
/// collision pole (dd_decoder §3.6).
const POLES: [(&str, [f32; 8], u32); 3] = [
    (
        "rho_ratio",
        [0.2, 0.55, 1.0, 0.0, 1.7, 3.1, 6.5, 12.0],
        0x7f80_0000,
    ),
    (
        "V_0",
        [-2.3, -1.8, -1.35, 0.0, -0.95, -0.6, -0.4, -0.15],
        0xff80_0000,
    ),
    (
        "E_0",
        [-1.25, -0.8, -0.35, 0.0, 0.2, 0.55, 0.9, 1.15],
        0xff80_0000,
    ),
];

/// `field`'s pole scene, sample [`POLE`] holding the f32 bits `bits`.
fn pole_scene(field: &'static str, values: [f32; 8], bits: u32) -> Scene {
    let mut s = row_scene(field, 8, Colouring::View(field)).unwrap_or_else(|e| panic!("{e}"));
    for (i, v) in (0u32..).zip(values) {
        let v = if i == POLE { f32::from_bits(bits) } else { v };
        match field {
            "rho_ratio" => s.set.ic(i).rho_ratio = v,
            "V_0" => s.set.ic(i).V_0 = v,
            _ => {
                s.set.sample(i).E_0(v);
            }
        }
    }
    s
}

/// The canonical quiet NaN's bits, the reference the pole's hatch is compared with.
const QNAN: u32 = 0x7fc0_0000;

/// Checks that `pole`, `field`'s scene at its pole value, and `nan`, the same scene with NaN in its place, render the
/// same image, bit for bit: the pole's tile the hatch, every other pixel unchanged, the measured range included; and
/// that the pole's sample stores the pole's bits, reads invalid on the twin, and the render is its twin.
fn check_pole(field: &str, (s, pole): (&Scene, &[[f32; 4]]), nan: &[[f32; 4]], bits: u32) {
    let (v, _) = s.value(POLE).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(v.to_bits(), bits, "`{field}`'s pole sample stores {v}");
    assert_eq!(look(s, POLE), Look::Invalid, "`{field}`'s pole's look");
    let width = s.size().0 as usize;
    for (k, (a, b)) in pole.iter().zip(nan).enumerate() {
        assert!(
            a.map(f32::to_bits) == b.map(f32::to_bits),
            "pixel ({}, {}) of `{field}` at {} is {a:?}, not the NaN's {b:?}",
            k % width,
            k / width,
            f32::from_bits(bits)
        );
    }
    assert_eq!(pole.len(), nan.len(), "`{field}`: the images' sizes");
    let image: Vec<Rgb> = pole
        .iter()
        .map(|p| [p[0], p[1], p[2]].map(f64::from))
        .collect();
    check_sample(s, &image, POLE, Look::Invalid, "the hatch");
    check_twin(s, &image);
}

/// `s` rendered with its colouring's WGSL as given, `None`, or with `from` replaced by `to`: the controls' wrong view.
fn render_view(h: &GpuHarness, s: &Scene, edit: Option<(&str, &str)>) -> Vec<[f32; 4]> {
    match edit {
        None => s.render(h.device(), h.queue()),
        Some((from, to)) => {
            let src = s.colour().unwrap_or_else(|e| panic!("{e}"));
            assert!(src.contains(from), "`{}`'s view has no `{from}`", s.name);
            s.render_colour(h.device(), h.queue(), &src.replace(from, to))
        }
    }
    .unwrap_or_else(|e| panic!("{e}"))
}

/// Checks every pole scene, each view rendered with `edit` applied (`None` for the views as generated).
fn check_poles(edit: Option<(&str, &str)>) {
    let h = gpu();
    for (field, values, bits) in POLES {
        let s = pole_scene(field, values, bits);
        let nan = pole_scene(field, values, QNAN);
        let (a, b) = (render_view(&h, &s, edit), render_view(&h, &nan, None));
        check_pole(field, (&s, &a), &b, bits);
    }
}

#[test]
fn non_finite_poles_draw_the_hatch_as_nan_does() {
    check_poles(None);
}

/// The non-finite guard as the views write it, and the NaN-alone guard TASK-M1-09 built: the controls' edit.
#[cfg(feature = "controls")]
const GUARD_TEST: (&str, &str) = (
    "(bitcast<u32>(raw) & 0x7f800000u) == 0x7f800000u",
    "bitcast<u32>(raw) == 0x7fc00000u",
);

negative_control!(
    non_finite_poles_draw_the_hatch_as_nan_does,
    "a guard that tests NaN alone draws `rho_ratio`'s +∞ on the ramp, not the hatch",
    expected = "of `rho_ratio` at inf is",
    check_poles(Some(GUARD_TEST))
);

/// Checks that samples 0 and 1 of `s`, the `d_min` scene, unset, render in `view` the "not yet" style bit for bit as
/// in `ramp`, the same scene through `d_min`'s field ramp, which tests its unset value first.
fn check_unset_order(s: &Scene, view: &[[f32; 4]], ramp: &[[f32; 4]]) {
    let width = s.size().0;
    for i in [0, 1] {
        assert_eq!(look(s, i), Look::NotYet, "sample {i}'s look");
        for (x, y) in s.pixels(i) {
            let k = (y * width + x) as usize;
            assert!(
                view[k].map(f32::to_bits) == ramp[k].map(f32::to_bits),
                "pixel ({x}, {y}) of the unset `d_min` is {:?}, not the \"not yet\" style {:?}",
                view[k],
                ramp[k]
            );
        }
    }
}

/// `d_min`'s scene, and the same through its field ramp.
fn d_min_and_ramp() -> (Scene, Scene) {
    let s = named("d_min");
    let mut r = named("d_min");
    r.colouring = Colouring::Ramp {
        ramp: FieldRamp::d_min().unwrap_or_else(|e| panic!("{e}")),
        override_on: false,
    };
    (s, r)
}

#[test]
fn non_finite_d_min_unset_stays_not_yet() {
    let h = gpu();
    let (s, r) = d_min_and_ramp();
    let (view, ramp) = (render_view(&h, &s, None), render_view(&h, &r, None));
    check_unset_order(&s, &view, &ramp);
}

negative_control!(
    non_finite_d_min_unset_stays_not_yet,
    "the guard before `d_min`'s unset line draws its +∞ as the hatch",
    expected = "of the unset `d_min` is",
    {
        let h = gpu();
        let (s, r) = d_min_and_ramp();
        let unset =
            "    if (bitcast<u32>(raw) == 0x7f800000u) { return debug_not_yet(ctx.frag_xy); }\n";
        let src = s.colour().unwrap_or_else(|e| panic!("{e}"));
        let guard = src
            .lines()
            .find(|l| l.contains(GUARD_TEST.0))
            .map(|l| format!("{l}\n"))
            .unwrap_or_else(|| panic!("no guard in `d_min`'s view"));
        let swapped = src
            .replace(unset, "@@")
            .replace(&guard, unset)
            .replace("@@", &guard);
        let view = s
            .render_colour(h.device(), h.queue(), &swapped)
            .unwrap_or_else(|e| panic!("{e}"));
        check_unset_order(&s, &view, &render_view(&h, &r, None));
    }
);
