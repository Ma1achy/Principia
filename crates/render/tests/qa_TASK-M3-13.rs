//! QA's tests for TASK-M3-13's render side (REQ-RENDER-028, R-17, R-245; REQ-PAY-098, R-400, R-426), written from
//! the requirements, not from the implementation:
//! - `qa_m3_13_diffusion_reads_t_end_step`: through the read side, the diffusion validity is the predicate `n ≥ 2` on
//!   the ledger's streaming-slope count, the sample's own `t_end_step`, never its other time field: `n = 0` and
//!   `n = 1` read the canonical NaN whatever `C_ty` holds, and `n ≥ 2` reads `C_ty / C_tt(n)`,
//!   `C_tt(n) = h²·n(n² − 1)/12` (payload §4), evaluated here in f64, within the read's f32 roundings.
//! - `qa_m3_13_diffusion_view_hatches_below_two`: on the GPU, the diffusion view draws `n = 0` and `n = 1` with the
//!   invalid hatch (REQ-COL-055) even under a large `C_ty`, and draws `n ≥ 2` off the hatch.
//! - `qa_m3_13_mean_y_view_viridis_fixed`: on the GPU, the checked-in `mean_y` view draws a value `v` in `[0, 4]` as
//!   `ramp_viridis(v / 4)`, its declared range in fixed mode (R-400; REQ-PAY-098), not on `ramp_coolwarm`.
//!
//! Tolerance: 1e-5 per linear-RGB channel for a GPU colour against its f64 CPU colour, parity_contract §4's ~1e-5
//! starting figure for one GPU evaluation (R-85). Each test has its negative control (R-176).

use render::present::{self, Rgb};
use validation::golden_scene::{scene, Colouring, Scene};
use validation::gpu::GpuHarness;
use validation::negative_control;

const QNAN: u32 = 0x7fc0_0000;
const U32: f64 = f32::EPSILON as f64 / 2.0;
const TOL: f64 = 1e-5;

/// Per sample: `t_end_step`, the other time field `t_dmin_step`, and `C_ty`.
const SAMPLES: [(u32, u32, f32); 8] = [
    (0, 7, 3.5e2),
    (1, 9, -2.0e-1),
    (2, 1, 4.0e-6),
    (3, 40, -7.5e-5),
    (11, 2, 2.2e-3),
    (97, 500, -1.3e-1),
    (512, 3, 9.0e1),
    (1000, 999, 4.4e2),
];

fn fixture() -> Scene {
    let mut s = scene("diffusion").unwrap_or_else(|e| panic!("{e}"));
    for (i, &(n, other, c)) in SAMPLES.iter().enumerate() {
        s.set.sample(i as u32).times(n, other).C_ty(c);
    }
    s.colouring = Colouring::View("diffusion");
    s
}

/// `C_tt(n)` in f64 (payload §4).
fn c_tt(n: u32, h: f64) -> f64 {
    let n = f64::from(n);
    h * h * n * (n * n - 1.0) / 12.0
}

/// Checks each sample's read against the slope over the count `count(t_end_step, t_dmin_step)`: NaN below 2, else
/// `C_ty/C_tt` within `7u` relative (five roundings of `C_tt`, its division, and the f32 `h`'s square is exact input).
fn check_reads(s: &Scene, count: fn(u32, u32) -> u32) {
    let h = f64::from(s.context.dt_macro);
    for (i, &(n_end, other, c)) in SAMPLES.iter().enumerate() {
        let n = count(n_end, other);
        let r = s.read(i as u32);
        if n < 2 {
            assert!(!r.diffusion_slope_valid, "sample {i}, n = {n}: valid");
            assert_eq!(r.diffusion.to_bits(), QNAN, "sample {i}, n = {n}: not NaN");
        } else {
            let want = f64::from(c) / c_tt(n, h);
            assert!(r.diffusion_slope_valid, "sample {i}, n = {n}: invalid");
            assert!(
                (f64::from(r.diffusion) - want).abs() <= 7.0 * U32 * want.abs(),
                "sample {i}, n = {n}: reads {}, not C_ty/C_tt(n) = {want}",
                r.diffusion
            );
        }
    }
}

#[test]
fn qa_m3_13_diffusion_reads_t_end_step() {
    check_reads(&fixture(), |n, _| n);
}

negative_control!(
    qa_m3_13_diffusion_reads_t_end_step,
    "the slope over the sample's other time field, t_dmin_step, is not what it reads",
    expected = "sample 0, n = 7",
    check_reads(&fixture(), |_, other| other)
);

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

fn render(s: &Scene) -> Vec<Rgb> {
    let h = gpu();
    s.render(h.device(), h.queue())
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .map(|p| [p[0], p[1], p[2]].map(f64::from))
        .collect()
}

fn near(a: Rgb, b: Rgb) -> bool {
    a.iter().zip(&b).all(|(x, y)| (x - y).abs() <= TOL)
}

fn hatch(x: u32, y: u32) -> Rgb {
    present::debug_invalid([f64::from(x) + 0.5, f64::from(y) + 0.5])
}

/// Checks that the samples `hatched` are drawn with the hatch at every pixel, and every other sample off it at some.
fn check_hatch(s: &Scene, image: &[Rgb], hatched: fn(u32) -> bool) {
    let width = s.size().0;
    for (i, &(n, ..)) in SAMPLES.iter().enumerate() {
        let px = s.pixels(i as u32);
        let on = px
            .iter()
            .filter(|&&(x, y)| near(image[(y * width + x) as usize], hatch(x, y)))
            .count();
        if hatched(n) {
            assert_eq!(on, px.len(), "sample {i}, n = {n}: not hatched");
        } else {
            assert!(on < px.len(), "sample {i}, n = {n}: hatched");
        }
    }
}

#[test]
fn qa_m3_13_diffusion_view_hatches_below_two() {
    let s = fixture();
    check_hatch(&s, &render(&s), |n| n < 2);
}

negative_control!(
    qa_m3_13_diffusion_view_hatches_below_two,
    "n = 2, the first valid fit, is not hatched",
    expected = "n = 2: not hatched",
    {
        let s = fixture();
        check_hatch(&s, &render(&s), |n| n < 3);
    }
);

/// `mean_y` values across its declared range, both ends included.
const MEAN_Y: [f32; 8] = [0.0, 0.3, 0.9, 1.5, 2.0, 2.7, 3.4, 4.0];

fn mean_y_scene() -> Scene {
    let mut s = scene("diffusion").unwrap_or_else(|e| panic!("{e}"));
    for (i, &v) in MEAN_Y.iter().enumerate() {
        s.set.sample(i as u32).times(10, 0).mean_y(v);
    }
    s.colouring = Colouring::View("mean_y");
    s
}

/// Checks every pixel of sample `i` against `want(v)`.
fn check_mean_y(s: &Scene, image: &[Rgb], want: fn(f64) -> Rgb) {
    let width = s.size().0;
    for (i, &v) in MEAN_Y.iter().enumerate() {
        let c = want(f64::from(v));
        for (x, y) in s.pixels(i as u32) {
            let got = image[(y * width + x) as usize];
            assert!(near(got, c), "mean_y = {v} draws {got:?}, not {c:?}");
        }
    }
}

#[test]
fn qa_m3_13_mean_y_view_viridis_fixed() {
    let s = mean_y_scene();
    check_mean_y(&s, &render(&s), |v| present::ramp_viridis(v / 4.0));
}

negative_control!(
    qa_m3_13_mean_y_view_viridis_fixed,
    "R-426's provisional cool-warm over the symmetric [−4, 4] is not what the view draws",
    expected = "draws",
    {
        let s = mean_y_scene();
        check_mean_y(&s, &render(&s), |v| present::ramp_coolwarm((v + 4.0) / 8.0));
    }
);
