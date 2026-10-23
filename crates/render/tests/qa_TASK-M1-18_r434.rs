//! QA tests for the M1 gate's colour confirmations, R-433 and R-434, written from REQ-COL-053, REQ-COL-062, REQ-COL-064,
//! REQ-TOOL-152 and colour_composition §1.4 / the render contract's stipple bullet, not from the implementation. Colours
//! are decoded with IEC 61966-2-1's sRGB transfer and taken to OKLab with Ottosson's matrices, both typed in here.
//! - REQ-COL-053 / REQ-COL-064 (R-433, R-434): an unset `d_min`, rendered, draws `#4E4E4E` with a 2 × 2 px dot of
//!   `#909090` in each 4 × 4 physical-pixel cell, where bit 1 of `x` and of `y` are both 0, and no other colour
//!   (`r434_qa_stipple_*`). Amended by qa for TASK-M2-32, as R-446 forces: a running sample in the outcome palette
//!   leaves the style's scope and draws bounded / nothing yet's `#141418`. Amended by qa for TASK-M2-34, as R-399
//!   ("the hatch stays NaN's and `sim_failed`'s"), R-467 and R-511 Z3 force: in `d_min`'s field ramp, a presentation
//!   view, the forced failures holding +∞ are `sim_failed` and draw `#141418` whatever their payload; the view keeps
//!   the stipple on every sample.
//! - REQ-COL-064 (R-434, the bar): no flat colour comes within τ = 0.101, the OKLab distance from `#FF00FF` to
//!   `#E034C6` (R-429), of every pixel of a 4 × 4 block. Every block holds both colours, so a flat colour within τ of
//!   all its pixels is within τ of the grey and of the dot, and by the triangle inequality the two would be at most
//!   2τ apart. The check is that they are more than 2τ apart, which holds for every colour of every palette at once,
//!   and fails for the proposal's `#808080` dot, 0.176 from the grey (`r434_qa_no_flat_colour_*`).
//! - REQ-COL-062 (R-433, R-434): each triple-outcome swatch is farther than τ from the dot: R-433's `#D6A1FF` and
//!   `#000097`, and since R-445 the occupant's `#F29620` and `#611597` (amended by qa for TASK-M2-32)
//!   (`r434_qa_triple_swatches_*`).
//! - REQ-TOOL-152 (R-433): the banding bound is 1/16 of a step (`r433_qa_banding_bound_*`).
//!
//! Tolerance on rendered pixels: 1e-5 per linear-RGB channel, parity_contract §4's ~1e-5 starting figure for one GPU
//! evaluation against its f64 reference (R-85). Each test has a registered negative control (R-176).

// The file name `qa_TASK-M1-18_r434` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use render::colour::field_ramp::FieldRamp;
use validation::golden_scene::{scene, Colouring, Scene, STATE_SIM_FAILED};
use validation::gpu::GpuHarness;
use validation::negative_control;
use validation::synthetic::Synthetic;

const TOL: f64 = 1e-5;

/// Payload §2's running state.
#[cfg(feature = "controls")]
const STATE_RUNNING: u32 = 3;

/// R-433's running grey and R-434's dot, 8-bit sRGB.
const GREY: u32 = 0x4E4E4E;
const DOT: u32 = 0x909090;

/// The proposal's dot, which R-434 replaced.
const PROPOSED_DOT: u32 = 0x808080;

/// R-433's triple-outcome swatches, superseded by R-445: the record.
const M1_TRIPLE_COLLISION: u32 = 0xD6A1FF;
const M1_TRIPLE_EJECTION: u32 = 0x000097;

/// R-445's triple-outcome swatches.
const TRIPLE_COLLISION: u32 = 0xF29620;
const TRIPLE_EJECTION: u32 = 0x611597;

type Rgb = [f64; 3];

fn rgb8(h: u32) -> [u8; 3] {
    [(h >> 16) as u8, (h >> 8) as u8, h as u8]
}

/// IEC 61966-2-1's sRGB decode of an 8-bit channel.
fn decode8(c: u8) -> f64 {
    let v = f64::from(c) / 255.0;
    if v <= 0.040_45 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

fn linear(h: u32) -> Rgb {
    rgb8(h).map(decode8)
}

/// Ottosson's linear sRGB to OKLab.
fn oklab(h: u32) -> Rgb {
    let [r, g, b] = linear(h);
    let l = 0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b;
    let m = 0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b;
    let s = 0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b;
    let [l, m, s] = [l.cbrt(), m.cbrt(), s.cbrt()];
    [
        0.210_454_255_3 * l + 0.793_617_785_0 * m - 0.004_072_046_8 * s,
        1.977_998_495_1 * l - 2.428_592_205_0 * m + 0.450_593_709_9 * s,
        0.025_904_037_1 * l + 0.782_808_380_0 * m - 0.808_675_766_0 * s,
    ]
}

fn dist(a: Rgb, b: Rgb) -> f64 {
    a.iter()
        .zip(&b)
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f64>()
        .sqrt()
}

/// τ: the OKLab distance from `#FF00FF` to `#E034C6` (R-429, R-132).
fn tau() -> f64 {
    dist(oklab(0xFF00FF), oklab(0xE034C6))
}

// ── REQ-COL-053 / REQ-COL-064: the rendered stipple ─────────────────────────────────────────────────────────────────

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

/// A rendered image: its width, height and linear RGB pixels, row-major.
struct Image {
    w: u32,
    h: u32,
    px: Vec<Rgb>,
}

fn render(h: &GpuHarness, s: &Scene) -> Image {
    let (w, hh) = s.size();
    let px = s
        .render(h.device(), h.queue())
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .map(|p| [p[0], p[1], p[2]].map(f64::from))
        .collect();
    Image { w, h: hh, px }
}

/// The harness's eight-sample row, every `d_min` unset: even samples forced failures holding +∞, odd ones unstepped
/// (R-271).
fn unset_d_min(colouring: Colouring) -> Scene {
    let mut s = scene("length_view").unwrap_or_else(|e| panic!("{e}"));
    s.set = Synthetic::flat(s.context.grid, 0);
    for i in 0..s.context.grid.sample_count() as u32 {
        if i % 2 == 0 {
            s.set
                .sample(i)
                .state(STATE_SIM_FAILED)
                .times(40, 0)
                .d_min(f32::INFINITY);
        } else {
            s.set.sample(i).times(0, 0);
        }
    }
    s.colouring = colouring;
    s
}

/// A row of running samples in the outcome palette.
#[cfg(feature = "controls")]
fn running() -> Scene {
    let mut s = scene("detail_view").unwrap_or_else(|e| panic!("{e}"));
    for i in 0..s.context.grid.sample_count() as u32 {
        s.set
            .sample(i)
            .state(STATE_RUNNING)
            .detail(i % 4)
            .times(i * 7, 0);
    }
    s.colouring = Colouring::Outcome { edit: None };
    s
}

fn near(a: Rgb, b: Rgb) -> bool {
    a.iter().zip(&b).all(|(x, y)| (x - y).abs() <= TOL)
}

/// Every pixel of `img`, `what`, is `dot` where bit 1 of `x` and of `y` are both 0, and `grey` elsewhere.
#[cfg(feature = "controls")]
fn check_stipple(img: &Image, what: &str, grey: u32, dot: u32) {
    check_stipple_in(img, what, grey, dot, &[]);
}

/// [`check_stipple`], but a pixel `failed` marks, a forced failure's in a presentation view, is bounded / nothing
/// yet's `#141418` (R-399, R-467, R-511 Z3; amended by qa for TASK-M2-34).
fn check_stipple_in(img: &Image, what: &str, grey: u32, dot: u32, failed: &[bool]) {
    assert!(img.w >= 4 && img.h >= 4, "{what}: smaller than one cell");
    for y in 0..img.h {
        for x in 0..img.w {
            let got = img.px[(y * img.w + x) as usize];
            let (want, name) = if failed.get((y * img.w + x) as usize) == Some(&true) {
                (0x141418, "forced failure's")
            } else if (x | y) & 2 == 0 {
                (dot, "dot")
            } else {
                (grey, "grey")
            };
            assert!(
                near(got, linear(want)),
                "{what}: pixel ({x}, {y}) is {got:?}, not the {name} #{want:06X}"
            );
        }
    }
}

/// Which pixels of [`unset_d_min`]'s row are its forced failures', `sim_failed` (added by qa for TASK-M2-34).
fn failed_pixels() -> Vec<bool> {
    let s = unset_d_min(Colouring::View("d_min"));
    let (w, h) = s.size();
    let mut out = vec![false; (w * h) as usize];
    for i in 0..s.context.grid.sample_count() {
        if s.read(i).state == STATE_SIM_FAILED {
            for (x, y) in s.pixels(i) {
                out[(y * w + x) as usize] = true;
            }
        }
    }
    assert!(
        out.iter().any(|f| *f) && out.iter().any(|f| !*f),
        "the row holds no forced failure or no unstepped sample"
    );
    out
}

/// The renders that draw the style: an unset `d_min` in its view and its ramp, with the pixels that draw `#141418`
/// instead, the ramp's forced failures (R-399, R-467, R-511 Z3; amended by qa for TASK-M2-34). The outcome palette's
/// running samples left the style's scope (R-446).
fn renders(h: &GpuHarness) -> [(Image, &'static str, Vec<bool>); 2] {
    let ramp = FieldRamp::d_min().unwrap_or_else(|e| panic!("{e}"));
    [
        (
            render(h, &unset_d_min(Colouring::View("d_min"))),
            "the d_min view, d_min unset",
            Vec::new(),
        ),
        (
            render(
                h,
                &unset_d_min(Colouring::Ramp {
                    ramp,
                    override_on: false,
                }),
            ),
            "the d_min ramp, d_min unset",
            failed_pixels(),
        ),
    ]
}

#[test]
fn r434_qa_stipple_is_909090_dots_on_4e4e4e() {
    let h = gpu();
    for (img, what, failed) in renders(&h) {
        check_stipple_in(&img, what, GREY, DOT, &failed);
    }
}

negative_control!(
    r434_qa_stipple_is_909090_dots_on_4e4e4e,
    "the render still drawing the proposal's #808080 dots is caught",
    expected = "not the dot #808080",
    {
        let h = gpu();
        for (img, what, failed) in renders(&h) {
            check_stipple_in(&img, what, GREY, PROPOSED_DOT, &failed);
        }
    }
);

/// R-446's scope can fail: the outcome palette's running samples are not the stipple, so reading them as it fails.
#[cfg(feature = "controls")]
mod r434_qa_stipple_outcome_out_of_scope {
    use super::*;
    negative_control!(
        r434_qa_stipple_is_909090_dots_on_4e4e4e,
        "the outcome palette's running samples, #141418 since R-446, are not the stipple",
        expected = "the outcome palette, running: pixel (0, 0)",
        {
            let h = gpu();
            check_stipple(
                &render(&h, &running()),
                "the outcome palette, running",
                GREY,
                DOT,
            );
        }
    );
}

/// R-399, R-467 and R-511 Z3's leg can fail: the ramp's forced failures read as the stipple, as if their +∞ were read,
/// fail (added by qa for TASK-M2-34).
#[cfg(feature = "controls")]
mod r434_qa_stipple_ramp_failures_stippled {
    use super::*;
    negative_control!(
        r434_qa_stipple_is_909090_dots_on_4e4e4e,
        "the d_min ramp's forced failures read as the stipple are caught",
        expected = "the d_min ramp, d_min unset: pixel (0, 0)",
        {
            let h = gpu();
            let [_, (img, what, _)] = renders(&h);
            check_stipple(&img, what, GREY, DOT);
        }
    );
}

#[cfg(feature = "controls")]
mod r434_qa_stipple_phase {
    use super::*;
    negative_control!(
        r434_qa_stipple_is_909090_dots_on_4e4e4e,
        "the render read with the grey and dot swapped, dots at every other phase, is caught",
        expected = "not the dot #4E4E4E",
        {
            let h = gpu();
            for (img, what, failed) in renders(&h) {
                check_stipple_in(&img, what, DOT, GREY, &failed);
            }
        }
    );
}

// ── REQ-COL-064: the bar ─────────────────────────────────────────────────────────────────────────────────────────

/// No colour at all comes within τ of both `grey` and `dot`: they are more than 2τ apart in OKLab. Every 4 × 4 block,
/// at every phase, holds four dot pixels and twelve grey ones, which is checked first.
fn check_no_flat_colour(grey: u32, dot: u32) {
    for oy in 0..4u32 {
        for ox in 0..4u32 {
            let dots = (0..16u32)
                .filter(|k| ((ox + k % 4) | (oy + k / 4)) & 2 == 0)
                .count();
            assert_eq!(dots, 4, "the block at phase ({ox}, {oy}) has {dots} dots");
        }
    }
    let d = dist(oklab(grey), oklab(dot));
    let t = tau();
    assert!((t - 0.101).abs() < 5e-4, "τ is {t:.4}, not R-429's 0.101");
    println!(
        "R-434: #{grey:06X} to #{dot:06X} is {d:.4} in OKLab; any flat colour is at least {:.4} from one of them, \
         τ = {t:.4}",
        d / 2.0
    );
    assert!(
        d / 2.0 > t,
        "#{grey:06X} and #{dot:06X} are {d:.4} apart, so a flat colour midway is within τ = {t:.4} of every pixel \
         of a 4 × 4 block"
    );
}

#[test]
fn r434_qa_no_flat_colour_within_tau_of_a_block() {
    check_no_flat_colour(GREY, DOT);
    // R-434's reason for the change: with the proposal's dots a colour midway, the grey ramp's, came within τ.
    let d = dist(oklab(GREY), oklab(PROPOSED_DOT));
    assert!(
        d / 2.0 <= tau(),
        "the proposal's #{PROPOSED_DOT:06X} dots already cleared τ ({:.4} each side)",
        d / 2.0
    );
}

negative_control!(
    r434_qa_no_flat_colour_within_tau_of_a_block,
    "the proposal's #808080 dots, 0.176 from the grey, leave a flat colour within τ of a block",
    expected = "so a flat colour midway is within τ",
    check_no_flat_colour(GREY, PROPOSED_DOT)
);

// ── REQ-COL-062: the triple swatches stay apart from the dot ─────────────────────────────────────────────────────

/// Each of `swatches` is farther than τ from the dot.
fn check_apart_from_dot(swatches: &[u32]) {
    let t = tau();
    for &s in swatches {
        let d = dist(oklab(s), oklab(DOT));
        assert!(
            d > t,
            "#{s:06X} is {d:.4} from the dot #{DOT:06X}, within τ = {t:.4}"
        );
    }
}

#[test]
fn r434_qa_triple_swatches_apart_from_the_dot() {
    check_apart_from_dot(&[M1_TRIPLE_COLLISION, M1_TRIPLE_EJECTION]);
    check_apart_from_dot(&[TRIPLE_COLLISION, TRIPLE_EJECTION]);
    // The swatches the occupant declares are R-445's, which supersede R-433's.
    let declared: Vec<[u8; 3]> = render::colour::outcome::SWATCHES
        .iter()
        .filter(|s| s.param.starts_with("triple_"))
        .map(|s| s.srgb8)
        .collect();
    assert_eq!(declared, [rgb8(TRIPLE_COLLISION), rgb8(TRIPLE_EJECTION)]);
}

negative_control!(
    r434_qa_triple_swatches_apart_from_the_dot,
    "a light grey #9A9A9A is within τ of the dot",
    expected = "from the dot #909090, within",
    check_apart_from_dot(&[0x9A9A9A])
);

// ── REQ-TOOL-152: the banding bound ──────────────────────────────────────────────────────────────────────────────

fn check_banding_bound(b: f64) {
    assert_eq!(b, 1.0 / 16.0, "the banding bound is not 1/16 of a step");
}

#[test]
fn r433_qa_banding_bound_is_one_sixteenth() {
    check_banding_bound(render::coords::BANDING_BOUND);
}

negative_control!(
    r433_qa_banding_bound_is_one_sixteenth,
    "1/8 of a step is not R-433's bound",
    expected = "the banding bound is not 1/16",
    check_banding_bound(0.125)
);
