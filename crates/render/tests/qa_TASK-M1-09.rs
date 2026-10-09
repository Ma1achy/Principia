//! QA tests for TASK-M1-09 on the render side, written from the requirements and their sources, not from the
//! implementation. Each draws a scene of its own (eight samples in a row, one 8 px tile each) through the render
//! harness (`validation::golden_scene::Scene::render`, `render::bind::preset_module` and `upload`) and checks every
//! pixel against a colour computed here from the requirement: the ramp place from the field's stored value by RQ-231's
//! compaction (log `1 − 1/(1 + ln(1 + |x|/ε))` with REQ-TOOL-160's proposed ε = 2⁻²⁴; cyclic `fract(x/2π)`,
//! REQ-TOOL-161; lin the identity), normalised by the ledger's fixed range or, under `RANGE_AUTO = 1`, by the min and
//! max of the compacted values that reach the ramp (render_gui_spec §10.1's `meas`), drawn through the presentation
//! layer's CPU mirrors (`render::present`, TASK-M1-03's). The emitter's own CPU twin (`NumericView::shown`,
//! `Scene::look`) is never used.
//! - REQ-RENDER-023 (R-79): a failed sample's stored 0.0 (`dE_max`, `dLz_max`, `d_min`) renders the ramp colour of
//!   0.0, not the hatch (`qa_debug_fields_raw_*`).
//! - REQ-GEN-012 (R-136, R-245): a diffusion with n < 2 reads the canonical NaN and renders the hatch, distinct from
//!   every valid slope on the ramp; `dmin_pair`'s 3 and the word `length`'s 127 round-trip bit-exact and render as
//!   their literal values, `dbg_sentinel(3)` and `dbg_sentinel(127)` (`qa_diffusion_invalid_fit_*`).
//! - REQ-TOOL-023 (R-136, R-280): `d_min`, `dE_max` and `dLz_max` round-trip within f16 eps, 2⁻¹⁰ relative
//!   (`qa_f16_scalar_views_*`).
//! - REQ-TOOL-137 / REQ-TOOL-012 (R-271, R-280, R-343 item 5): `d_min`'s unset value, f16 +∞, of a forced failure and
//!   of an unstepped sample is drawn in the "not yet" grey, neither hatched nor on the ramp; the largest finite f16
//!   is on the ramp; a NaN in the same view is hatched; `ftle` with `ftle_valid` false is hatched
//!   (`qa_f16_scalar_views_d_min_*`, `qa_debug_fields_raw_ftle_*`).
//! - REQ-COL-001 (R-132): a field ramp draws its invalid pixels (the word `length`'s sentinel 127, `d_min`'s NaN) in
//!   the hatched pattern; the node's override changes those pixels alone, to its declared colour; `d_min`'s unset
//!   value stays grey either way (`qa_field_ramp_*`).
//! - REQ-TOOL-161: `rho_angle`'s view places angles a whole number of turns apart, negative ones included, at one
//!   place of `ramp_twilight` (`qa_cyclic_*`).
//!
//! Tolerance: 1e-5 per linear-RGB channel, parity_contract §4's ~1e-5 starting figure for one GPU evaluation against
//! its f64 reference (R-85). Each test has a registered negative control (R-176).

use render::assemble::Declaration;
use render::colour::field_ramp::FieldRamp;
use render::present::{self, Rgb};
use validation::golden_scene::{fgw_length_raw, scene, Colouring, Scene, STATE_SIM_FAILED};
use validation::gpu::GpuHarness;
use validation::negative_control;
use validation::synthetic::Synthetic;

const TOL: f64 = 1e-5;
const TILE: u32 = 8;
const SAMPLES: u32 = 8;
const QNAN: u32 = 0x7FC0_0000;
/// f16 +∞ (R-271) and the f16 quiet NaN, which `unpack2x16float` widens to the canonical f32 quiet NaN.
const F16_INF: u32 = 0x7C00;
const F16_QNAN: u32 = 0x7E00;

/// REQ-TOOL-160's proposed ε.
fn eps() -> f64 {
    2f64.powi(-24)
}

/// RQ-231's log compaction, `dbg_log`'s place.
fn log_place(x: f64, eps: f64) -> f64 {
    1.0 - 1.0 / (1.0 + (1.0 + x.abs() / eps).ln())
}

/// What a pixel must show.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Want {
    Hatch,
    Grey,
    Viridis(f64),
    Twilight(f64),
    Literal(f32),
    Flat(Rgb),
}

impl Want {
    fn at(self, x: u32, y: u32) -> Rgb {
        let frag = [f64::from(x) + 0.5, f64::from(y) + 0.5];
        match self {
            Want::Hatch => present::debug_invalid(frag),
            Want::Grey => present::not_yet(),
            Want::Viridis(t) => present::ramp_viridis(t),
            Want::Twilight(t) => present::ramp_twilight(t),
            Want::Literal(v) => present::dbg_sentinel(v, frag),
            Want::Flat(c) => c,
        }
    }
}

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

/// A fresh scene of eight samples coloured by `colouring`.
fn fresh(colouring: Colouring) -> Scene {
    let mut s = scene("length_view").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(s.size(), (SAMPLES * TILE, TILE), "the scene's row");
    s.set = Synthetic::flat(s.context.grid, 0);
    s.colouring = colouring;
    s
}

fn render(h: &GpuHarness, s: &Scene) -> Vec<Rgb> {
    s.render(h.device(), h.queue())
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .map(|p| [p[0], p[1], p[2]].map(f64::from))
        .collect()
}

fn near(a: Rgb, b: Rgb) -> bool {
    a.iter().zip(&b).all(|(x, y)| (x - y).abs() <= TOL)
}

/// Sample `i`'s pixels.
fn tile(i: u32) -> impl Iterator<Item = (u32, u32)> {
    (0..TILE).flat_map(move |y| (TILE * i..TILE * (i + 1)).map(move |x| (x, y)))
}

/// Checks that every pixel of sample `i` of `image` is `want`'s.
fn check(image: &[Rgb], i: u32, want: Want, what: &str) {
    for (x, y) in tile(i) {
        let got = image[(y * SAMPLES * TILE + x) as usize];
        let c = want.at(x, y);
        assert!(
            near(got, c),
            "pixel ({x}, {y}) of sample {i} is {got:?}, which is not {what} {want:?} = {c:?}"
        );
    }
}

/// `image` with sample `i` drawn as `want`: the controls' wrong renders.
#[cfg(feature = "controls")]
fn painted(image: &[Rgb], i: u32, want: Want) -> Vec<Rgb> {
    let mut out = image.to_vec();
    for (x, y) in tile(i) {
        out[(y * SAMPLES * TILE + x) as usize] = want.at(x, y);
    }
    out
}

/// The auto-range places of `raws` (`None` for a sample off the ramp): `(raw − min)/(max − min)` over the ramp's.
fn auto_places(raws: &[Option<f64>]) -> Vec<Option<f64>> {
    let on: Vec<f64> = raws.iter().flatten().copied().collect();
    let lo = on.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = on.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    raws.iter()
        .map(|r| r.map(|r| if hi > lo { (r - lo) / (hi - lo) } else { 0.0 }))
        .collect()
}

/// `packed_a` of sample `i` with its high half, `d_min`, written as the f16 bits `bits` (a bitwise-adversarial payload,
/// debug_tooling_plan "Synthetic-first").
fn d_min_bits(s: &mut Scene, i: u32, bits: u32) {
    let w = s.set.simstate(i).packed_a;
    s.set.sample(i).packed_a((w & 0xffff) | (bits << 16));
}

// ── REQ-RENDER-023: debug fields are raw ──────────────────────────────────────────────────────────────────────────

/// Stored drift maxima, each exact in f16; sample 0 is a failed sample holding 0.0.
const DRIFTS: [f32; 8] = [0.0, 0.0009765625, 0.25, 1.0, 8.0, 512.0, 0.5, 6.1035156e-5];

/// The `dE_max` or `dLz_max` scene: sample 0 failed with 0.0, the rest stepped with [`DRIFTS`].
fn drift_scene(field: &'static str) -> Scene {
    let mut s = fresh(Colouring::View(field));
    s.set
        .sample(0)
        .state(STATE_SIM_FAILED)
        .times(30, 0)
        .drift_max(0.0, 0.0);
    for i in 1..SAMPLES {
        let v = DRIFTS[i as usize];
        s.set.sample(i).times(30, 0).drift_max(v, v);
    }
    s
}

/// Checks each sample of a drift view against its log place under `eps` (RANGE_AUTO = 1: the ledger range is
/// unbounded), sample 0, failed, at 0.0's: the ramp's start, not the hatch.
fn check_drift(s: &Scene, image: &[Rgb], eps: f64) {
    let read = s.read(0);
    assert_eq!(read.state, STATE_SIM_FAILED, "sample 0 is failed");
    let raws: Vec<Option<f64>> = DRIFTS
        .iter()
        .map(|&v| Some(log_place(f64::from(v), eps)))
        .collect();
    let places = auto_places(&raws);
    assert_eq!(places[0], Some(0.0), "0.0 is the least place");
    for i in 0..SAMPLES {
        check(
            image,
            i,
            Want::Viridis(places[i as usize].unwrap()),
            "the ramp colour of its stored value",
        );
    }
}

#[test]
fn qa_debug_fields_raw_failed_zero_is_the_ramp_colour_of_zero() {
    let h = gpu();
    for f in ["dE_max", "dLz_max"] {
        let s = drift_scene(f);
        assert_eq!(s.read(0).dE_max.to_bits(), 0, "sample 0 stores 0.0");
        check_drift(&s, &render(&h, &s), eps());
    }
}

negative_control!(
    qa_debug_fields_raw_failed_zero_is_the_ramp_colour_of_zero,
    "a view masking the failed sample with the hatch is caught",
    expected = "is not the ramp colour of its stored value",
    {
        let s = drift_scene("dE_max");
        let image = painted(&render(&gpu(), &s), 0, Want::Hatch);
        check_drift(&s, &image, eps())
    }
);

/// The ε check can fail: the same render read against ε = 2⁻²³.
#[cfg(feature = "controls")]
mod qa_debug_fields_raw_eps {
    use super::*;
    negative_control!(
        qa_debug_fields_raw_failed_zero_is_the_ramp_colour_of_zero,
        "the render checked against ε = 2^-23 fails",
        expected = "is not the ramp colour of its stored value",
        {
            let s = drift_scene("dLz_max");
            check_drift(&s, &render(&gpu(), &s), 2f64.powi(-23))
        }
    );
}

/// `ftle`'s view: the unstepped sample (`ftle_valid` false, R-253) is hatched; the rest on the
/// ramp at their read value's auto place.
fn ftle_scene() -> Scene {
    let mut s = fresh(Colouring::View("ftle"));
    s.set.sample(0).times(0, 0);
    for (i, sv) in [
        (1u32, 0.2f32),
        (2, 0.9),
        (3, 1.7),
        (4, 2.4),
        (5, 3.3),
        (6, 0.05),
        (7, 1.1),
    ] {
        let mut sh = [[0.0f32; 2]; 3];
        sh[0][0] = 1e-6;
        s.set.sample(i).times(100, 0).S(sv).r_sh(sh);
    }
    s
}

fn check_ftle(s: &Scene, image: &[Rgb]) {
    let reads: Vec<_> = (0..SAMPLES).map(|i| s.read(i)).collect();
    assert!(!reads[0].ftle_valid, "the unstepped sample's ftle is valid");
    assert_eq!(
        reads[0].ftle.to_bits(),
        QNAN,
        "the unstepped sample's ftle is not the canonical NaN"
    );
    let raws: Vec<Option<f64>> = reads
        .iter()
        .map(|r| (r.ftle.to_bits() != QNAN).then_some(f64::from(r.ftle)))
        .collect();
    assert!(
        raws[1..].iter().all(Option::is_some),
        "a stepped sample's ftle reads NaN"
    );
    let places = auto_places(&raws);
    for i in 0..SAMPLES {
        match places[i as usize] {
            None => check(image, i, Want::Hatch, "the hatch of a NaN"),
            Some(t) => check(image, i, Want::Viridis(t), "the ramp colour of its value"),
        }
    }
}

#[test]
fn qa_debug_fields_raw_ftle_nan_is_hatched() {
    let s = ftle_scene();
    check_ftle(&s, &render(&gpu(), &s));
}

negative_control!(
    qa_debug_fields_raw_ftle_nan_is_hatched,
    "a NaN ftle drawn at the ramp's start is caught",
    expected = "is not the hatch of a NaN",
    {
        let s = ftle_scene();
        let image = painted(&render(&gpu(), &s), 0, Want::Viridis(0.0));
        check_ftle(&s, &image)
    }
);

// ── REQ-GEN-012: an invalid diffusion fit is NaN; stored sentinels show their literal value ───────────────────────

fn diffusion_scene() -> Scene {
    let mut s = fresh(Colouring::View("diffusion"));
    for (i, n, c) in [
        (0u32, 0u32, 2.0e-4f32),
        (1, 1, 2.0e-4),
        (2, 2, 4.0e-5),
        (3, 3, -9.0e-4),
        (4, 5, 3.0e-4),
        (5, 8, 1.1e-3),
        (6, 13, 2.6e-3),
        (7, 1, -5.0e-4),
    ] {
        s.set.sample(i).times(n, 0).C_ty(c);
    }
    s
}

/// Samples with n < 2 read the canonical NaN, invalid, and are hatched; every other is valid and on the ramp, at its
/// read slope's auto place, and differs from the hatch.
fn check_diffusion(s: &Scene, image: &[Rgb]) {
    let reads: Vec<_> = (0..SAMPLES).map(|i| s.read(i)).collect();
    let raws: Vec<Option<f64>> = reads
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let n_lt_2 = matches!(i, 0 | 1 | 7);
            assert_eq!(
                !r.diffusion_slope_valid, n_lt_2,
                "sample {i}'s fit validity (R-245)"
            );
            if n_lt_2 {
                assert_eq!(
                    r.diffusion.to_bits(),
                    QNAN,
                    "sample {i}, n < 2, does not read NaN"
                );
                None
            } else {
                assert!(
                    r.diffusion.is_finite(),
                    "sample {i}'s valid slope is {}",
                    r.diffusion
                );
                Some(f64::from(r.diffusion))
            }
        })
        .collect();
    let places = auto_places(&raws);
    for i in 0..SAMPLES {
        match places[i as usize] {
            None => check(image, i, Want::Hatch, "the hatch of an invalid fit"),
            Some(t) => {
                check(image, i, Want::Viridis(t), "the ramp colour of its slope");
                let (x, y) = tile(i).next().unwrap();
                assert!(
                    !near(Want::Hatch.at(x, y), Want::Viridis(t).at(x, y)),
                    "sample {i}'s valid slope is indistinguishable from the hatch"
                );
            }
        }
    }
}

#[test]
fn qa_diffusion_invalid_fit_reads_nan_and_is_hatched() {
    let s = diffusion_scene();
    check_diffusion(&s, &render(&gpu(), &s));
}

negative_control!(
    qa_diffusion_invalid_fit_reads_nan_and_is_hatched,
    "an n = 1 fit drawn as a −1.0 sentinel on the ramp (the retired R-17 reading) is caught",
    expected = "is not the hatch of an invalid fit",
    {
        let s = diffusion_scene();
        let image = painted(&render(&gpu(), &s), 1, Want::Literal(-1.0));
        check_diffusion(&s, &image)
    }
);

/// The word `length`'s view over lengths 0…76 and the sentinel 127, and `dmin_pair`'s over 0…2 and the sentinel 3.
const LENGTHS: [u32; 8] = [0, 11, 127, 30, 47, 76, 64, 127];
const PAIRS: [u32; 8] = [3, 0, 1, 2, 3, 2, 1, 0];

fn length_scene() -> Scene {
    let mut s = fresh(Colouring::View("length"));
    for (i, l) in LENGTHS.into_iter().enumerate() {
        s.set.sample(i as u32).word([0; 4], l);
    }
    s
}

fn pair_scene() -> Scene {
    let mut s = fresh(Colouring::View("dmin_pair"));
    for (i, p) in PAIRS.into_iter().enumerate() {
        s.set.sample(i as u32).dmin_pair(p);
    }
    s
}

/// The sentinels round-trip bit-exact and render as their literal values; the length view's other values sit on its
/// fixed `[0, 76]` (RANGE_AUTO = 0: the ledger range is bounded).
fn check_sentinels(length: (&Scene, &[Rgb]), pair: (&Scene, &[Rgb])) {
    let (s, image) = length;
    for (i, l) in LENGTHS.into_iter().enumerate() {
        let i = i as u32;
        assert_eq!(
            fgw_length_raw(s.set.word(i)),
            l,
            "the stored length of sample {i}"
        );
        assert_eq!(
            fgw_length_raw(s.read(i).word),
            l,
            "the read length of sample {i}"
        );
        if l == 127 {
            check(image, i, Want::Literal(127.0), "the literal sentinel");
        } else {
            check(
                image,
                i,
                Want::Viridis(f64::from(l) / 76.0),
                "the ramp colour of its length",
            );
        }
    }
    let (s, image) = pair;
    for (i, p) in PAIRS.into_iter().enumerate() {
        let i = i as u32;
        assert_eq!(s.read(i).dmin_pair, p, "the read dmin_pair of sample {i}");
        if p == 3 {
            check(image, i, Want::Literal(3.0), "the literal sentinel");
        }
    }
}

#[test]
fn qa_diffusion_invalid_fit_sentinels_round_trip_and_show_literally() {
    let h = gpu();
    let (l, p) = (length_scene(), pair_scene());
    let (a, b) = (render(&h, &l), render(&h, &p));
    check_sentinels((&l, &a), (&p, &b));
}

negative_control!(
    qa_diffusion_invalid_fit_sentinels_round_trip_and_show_literally,
    "a dmin_pair 3 drawn as a class rather than its literal value is caught",
    expected = "is not the literal sentinel",
    {
        let h = gpu();
        let (l, p) = (length_scene(), pair_scene());
        let (a, b) = (render(&h, &l), render(&h, &p));
        let b = painted(&b, 4, Want::Flat(present::dbg_cat(3, 3)));
        check_sentinels((&l, &a), (&p, &b))
    }
);

/// A second control: the length sentinel scaled onto the clamped ramp.
#[cfg(feature = "controls")]
mod qa_diffusion_invalid_fit_scaled {
    use super::*;
    negative_control!(
        qa_diffusion_invalid_fit_sentinels_round_trip_and_show_literally,
        "a length 127 scaled by the range (clamped to the ramp's end) is caught",
        expected = "is not the literal sentinel",
        {
            let h = gpu();
            let (l, p) = (length_scene(), pair_scene());
            let (a, b) = (render(&h, &l), render(&h, &p));
            let a = painted(&a, 2, Want::Viridis(1.0));
            check_sentinels((&l, &a), (&p, &b))
        }
    );
}

// ── REQ-TOOL-023: f16 scalars round-trip within f16 eps ───────────────────────────────────────────────────────────

/// IEEE 754 binary16's machine epsilon, 2⁻¹⁰.
const F16_EPS: f64 = 1.0 / 1024.0;

/// Values across f16's normal range, 2⁻¹⁴ … 65504, log-spaced and off the f16 grid.
fn f16_values() -> Vec<f32> {
    let (lo, hi) = (2f64.powi(-14).ln(), 65504f64.ln());
    (0..=400)
        .map(|k| (lo + (hi - lo) * f64::from(k) / 400.0).exp() as f32)
        .map(|v| v * 1.000_123)
        .filter(|v| *v <= 65504.0)
        .collect()
}

/// Each value written to `d_min`, `dE_max` and `dLz_max` reads back within `eps · |x|`.
fn check_f16(values: &[f32], eps: f64) {
    let mut s = fresh(Colouring::View("d_min"));
    for &v in values {
        s.set.sample(0).times(10, 0).d_min(v).drift_max(v, v);
        let r = s.read(0);
        for (name, got) in [
            ("d_min", r.d_min),
            ("dE_max", r.dE_max),
            ("dLz_max", r.dLz_max),
        ] {
            let err = (f64::from(got) - f64::from(v)).abs();
            assert!(
                err <= eps * f64::from(v).abs(),
                "{name} {v} reads back {got}, off by {err:e}, beyond f16 eps"
            );
        }
    }
}

#[test]
fn qa_f16_scalar_views_round_trip_within_f16_eps() {
    check_f16(&f16_values(), F16_EPS);
}

negative_control!(
    qa_f16_scalar_views_round_trip_within_f16_eps,
    "a tolerance of a hundredth of f16 eps, which f16 cannot hold, fails",
    expected = "beyond f16 eps",
    check_f16(&f16_values(), F16_EPS / 100.0)
);

// ── REQ-TOOL-137 / REQ-TOOL-012: d_min's unset value is grey, NaN hatched, the rest on the ramp ────────────────────

/// `d_min`'s scene: 0 a forced failure holding +∞; 1 unstepped (fresh); 2 an f16 NaN; 3 f16's largest finite value;
/// 4 a failed sample whose stored `d_min` is 0.0; 5–7 valid values.
fn d_min_scene(colouring: Colouring) -> Scene {
    let mut s = fresh(colouring);
    s.set
        .sample(0)
        .state(STATE_SIM_FAILED)
        .times(40, 0)
        .d_min(f32::INFINITY);
    s.set.sample(1).times(0, 0);
    s.set.sample(2).times(40, 0);
    d_min_bits(&mut s, 2, F16_QNAN);
    s.set.sample(3).times(40, 0).d_min(65504.0);
    s.set.sample(4).state(STATE_SIM_FAILED).times(40, 0);
    d_min_bits(&mut s, 4, 0);
    for (i, v) in [(5u32, 0.5f32), (6, 1.75), (7, 6.1035156e-5)] {
        s.set.sample(i).times(40, 0).d_min(v);
    }
    s
}

/// What sample `i` of [`d_min_scene`] holds, by its read bits: `None` for the unset value, `Some(None)` for NaN.
fn d_min_reads(s: &Scene) -> Vec<Option<Option<f64>>> {
    (0..SAMPLES)
        .map(|i| {
            let v = s.read(i).d_min;
            match v.to_bits() {
                0x7F80_0000 => None,
                QNAN => Some(None),
                _ => Some(Some(f64::from(v))),
            }
        })
        .collect()
}

fn check_d_min_view(s: &Scene, image: &[Rgb]) {
    let reads = d_min_reads(s);
    for i in [0u32, 1] {
        assert_eq!(
            s.set.simstate(i).packed_a >> 16,
            F16_INF,
            "sample {i} does not store f16 +inf"
        );
        assert_eq!(
            reads[i as usize], None,
            "sample {i} does not read the unset value"
        );
    }
    assert_eq!(
        reads[2],
        Some(None),
        "sample 2 does not read the canonical NaN"
    );
    assert_eq!(reads[4], Some(Some(0.0)), "sample 4 does not read 0.0");
    let raws: Vec<Option<f64>> = reads
        .iter()
        .map(|r| r.flatten().map(|v| log_place(v, eps())))
        .collect();
    let places = auto_places(&raws);
    for i in 0..SAMPLES {
        let want = match reads[i as usize] {
            None => Want::Grey,
            Some(None) => Want::Hatch,
            Some(Some(_)) => Want::Viridis(places[i as usize].unwrap()),
        };
        check(image, i, want, "d_min's look");
    }
    // The grey is neither the hatch nor a ramp colour of this view.
    let (x, y) = tile(0).next().unwrap();
    let grey = Want::Grey.at(x, y);
    for c in ledger::gen::prelude::hatch().colours.map(present::srgb8) {
        assert!(!near(grey, c), "the grey is a hatch colour");
    }
    for k in 0..=1000 {
        assert!(
            !near(grey, present::ramp_viridis(f64::from(k) / 1000.0)),
            "the grey sits on the ramp"
        );
    }
}

#[test]
fn qa_f16_scalar_views_d_min_unset_is_grey_nan_hatched() {
    let s = d_min_scene(Colouring::View("d_min"));
    check_d_min_view(&s, &render(&gpu(), &s));
}

negative_control!(
    qa_f16_scalar_views_d_min_unset_is_grey_nan_hatched,
    "the unstepped sample's unset d_min hatched as if NaN is caught",
    expected = "is not d_min's look",
    {
        let s = d_min_scene(Colouring::View("d_min"));
        let image = painted(&render(&gpu(), &s), 1, Want::Hatch);
        check_d_min_view(&s, &image)
    }
);

#[cfg(feature = "controls")]
mod qa_f16_scalar_views_unset_on_ramp {
    use super::*;
    negative_control!(
        qa_f16_scalar_views_d_min_unset_is_grey_nan_hatched,
        "the forced failure's unset d_min placed at the ramp's end, as +inf compacts, is caught",
        expected = "is not d_min's look",
        {
            let s = d_min_scene(Colouring::View("d_min"));
            let image = painted(&render(&gpu(), &s), 0, Want::Viridis(1.0));
            check_d_min_view(&s, &image)
        }
    );
}

// ── REQ-COL-001: the field ramps' invalid lane ────────────────────────────────────────────────────────────────────

/// A field ramp scene, its override on or off.
fn ramp_scene(field: &str, override_on: bool) -> Scene {
    let ramp = match field {
        "length" => FieldRamp::length(),
        _ => FieldRamp::d_min(),
    }
    .unwrap_or_else(|e| panic!("{e}"));
    let c = Colouring::Ramp { ramp, override_on };
    match field {
        "length" => {
            let mut s = fresh(c);
            for (i, l) in LENGTHS.into_iter().enumerate() {
                s.set.sample(i as u32).word([0; 4], l);
            }
            s
        }
        _ => d_min_scene(c),
    }
}

/// The override colour the ramp declares, `INVALID_COLOUR`'s default.
fn override_colour(s: &Scene) -> Rgb {
    let Colouring::Ramp { ramp, .. } = &s.colouring else {
        unreachable!()
    };
    let d = Declaration::parse(&ramp.wgsl()).unwrap_or_else(|e| panic!("{e:?}"));
    let u = d
        .uniforms
        .iter()
        .find(|u| u.name == "INVALID_COLOUR")
        .expect("the ramp declares INVALID_COLOUR");
    [u.default[0], u.default[1], u.default[2]]
}

/// The ramp's look of each sample with the override off, and which samples are invalid.
fn ramp_wants(field: &str, s: &Scene) -> Vec<(Want, bool)> {
    match field {
        "length" => LENGTHS
            .iter()
            .map(|&l| {
                if l == 127 {
                    (Want::Hatch, true)
                } else {
                    (Want::Viridis(f64::from(l) / 76.0), false)
                }
            })
            .collect(),
        _ => d_min_reads(s)
            .into_iter()
            .map(|r| match r {
                None => (Want::Grey, false),
                Some(None) => (Want::Hatch, true),
                Some(Some(v)) => (Want::Viridis((v / 2.0).clamp(0.0, 1.0)), false),
            })
            .collect(),
    }
}

/// With the override off, the invalid pixels are hatched and the rest on the ramp (or grey); with it on, the invalid
/// pixels take the declared override colour, which differs from the hatch, and no other pixel changes.
fn check_ramp(field: &str, off: (&Scene, &[Rgb]), on: (&Scene, &[Rgb])) {
    let wants = ramp_wants(field, off.0);
    assert!(
        wants.iter().any(|w| w.1),
        "`{field}`'s scene has no invalid sample"
    );
    let over = override_colour(on.0);
    for (i, &(want, invalid)) in wants.iter().enumerate() {
        let i = i as u32;
        check(off.1, i, want, &format!("`{field}`'s ramp look"));
        if invalid {
            check(
                on.1,
                i,
                Want::Flat(over),
                &format!("`{field}`'s override colour"),
            );
            for (x, y) in tile(i) {
                assert!(
                    !near(over, Want::Hatch.at(x, y)),
                    "`{field}`: the override colour is the hatch's"
                );
            }
        } else {
            for (x, y) in tile(i) {
                let k = (y * SAMPLES * TILE + x) as usize;
                assert!(
                    near(off.1[k], on.1[k]),
                    "`{field}`: the override changed valid pixel ({x}, {y}) of sample {i}"
                );
            }
        }
    }
}

#[test]
fn qa_field_ramp_invalid_lane_and_override() {
    let h = gpu();
    for field in ["length", "d_min"] {
        let (off, on) = (ramp_scene(field, false), ramp_scene(field, true));
        let (a, b) = (render(&h, &off), render(&h, &on));
        check_ramp(field, (&off, &a), (&on, &b));
    }
}

negative_control!(
    qa_field_ramp_invalid_lane_and_override,
    "an override that also recolours d_min's unset grey is caught",
    expected = "the override changed valid pixel",
    {
        let h = gpu();
        let (off, on) = (ramp_scene("d_min", false), ramp_scene("d_min", true));
        let (a, b) = (render(&h, &off), render(&h, &on));
        let over = override_colour(&on);
        let b = painted(&b, 1, Want::Flat(over));
        check_ramp("d_min", (&off, &a), (&on, &b))
    }
);

#[cfg(feature = "controls")]
mod qa_field_ramp_sentinel_on_ramp {
    use super::*;
    negative_control!(
        qa_field_ramp_invalid_lane_and_override,
        "a field ramp showing length's sentinel 127 literally, as a debug view does, is caught",
        expected = "is not `length`'s ramp look",
        {
            let h = gpu();
            let (off, on) = (ramp_scene("length", false), ramp_scene("length", true));
            let (a, b) = (render(&h, &off), render(&h, &on));
            let a = painted(&a, 2, Want::Literal(127.0));
            check_ramp("length", (&off, &a), (&on, &b))
        }
    );
}

// ── REQ-TOOL-161: the cyclic period is 2π ─────────────────────────────────────────────────────────────────────────

/// `rho_angle` values: a phase in turns and a whole number of turns added, negative ones included.
const ANGLES: [(f64, i32); 8] = [
    (0.1, 0),
    (0.3, 1),
    (0.55, -1),
    (0.8, 3),
    (0.95, -2),
    (0.75, -1),
    (0.42, 2),
    (0.2, -3),
];

fn cyclic_scene() -> Scene {
    let mut s = fresh(Colouring::View("rho_angle"));
    for (i, (phase, k)) in ANGLES.into_iter().enumerate() {
        s.set.ic(i as u32).rho_angle = ((phase + f64::from(k)) * std::f64::consts::TAU) as f32;
    }
    s
}

/// Each angle sits on `ramp_twilight` at its phase, `fract(x / period)` (RANGE_AUTO = 0: the fixed [0, 1]).
fn check_cyclic(image: &[Rgb], period: f64) {
    for (i, (phase, k)) in ANGLES.into_iter().enumerate() {
        let x = ((phase + f64::from(k)) * std::f64::consts::TAU) as f32;
        let t = (f64::from(x) / period).rem_euclid(1.0);
        check(
            image,
            i as u32,
            Want::Twilight(t),
            "the twilight colour of its phase",
        );
    }
}

#[test]
fn qa_cyclic_view_places_whole_turns_together() {
    let s = cyclic_scene();
    check_cyclic(&render(&gpu(), &s), std::f64::consts::TAU);
}

negative_control!(
    qa_cyclic_view_places_whole_turns_together,
    "the render read with a period of 360 (degrees) fails",
    expected = "is not the twilight colour of its phase",
    {
        let s = cyclic_scene();
        check_cyclic(&render(&gpu(), &s), 360.0)
    }
);
