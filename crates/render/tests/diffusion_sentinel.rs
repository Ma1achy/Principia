//! The diffusion view reads the ledger's streaming slope (REQ-RENDER-028; R-17, R-245; dd_simstate_payload §4, §5):
//! a fixture whose co-moments `mean_y` and `C_ty` are streamed through the kernel's Welford update
//! (`kernel::driver::welford`) from series of known slope, one per sample, at step counts `n = 0, 1, 2, 3, 10, 64,
//! 300, 1000`:
//! - `diffusion_sentinel_streaming_reads_the_slope`: through the read side, the predicate `n ≥ 2` reads the ledger's
//!   `n`, the sample's own `t_end_step`: `n = 0` and `n = 1` read the canonical NaN and are invalid, whatever `C_ty`
//!   holds, and every `n ≥ 2` is valid and reads its series' slope.
//! - `diffusion_sentinel_streaming_renders_the_hatch`: on the GPU, the view hatches `n = 0` and `n = 1` and draws
//!   every valid slope on its ramp, the image its CPU twin.
//!
//! Each test registers its negative control (R-176).

use render::present::Rgb;
use validation::golden_scene::{scene, Colouring, Look, Scene};
use validation::gpu::GpuHarness;
use validation::negative_control;
use validation::welford::Welford;

/// The canonical quiet NaN an invalid fit reads (R-245; lowering Part 3a).
const QNAN: u32 = 0x7fc0_0000;

/// The fixture's step counts, one per sample: R-245's two invalid counts, the first valid fit, and five more up to
/// the scenes' horizon of 1000.
const N: [u32; 8] = [0, 1, 2, 3, 10, 64, 300, 1000];

/// Each sample's series, `y_k = y₀ + b·t_k` at `t_k = k·h`, `k = 1 … n`: its start `y₀` and slope `b`, signed, with
/// every `y` in `[0, 4]`, the range of payload §4's `y` (a squared shape-sphere chord).
const SERIES: [(f32, f32); 8] = [
    (1.0, 7.0),
    (1.5, -9.0),
    (1.0, 25.0),
    (2.0, -40.0),
    (1.0, 12.0),
    (3.0, -3.0),
    (1.0, 0.8),
    (1.0, 0.2),
];

/// An update rule: the kernel's, or a control's faulty one.
type Update = fn(Welford<f32>, f32, u32, f32) -> Welford<f32>;

fn kernel_update(w: Welford<f32>, y: f32, n: u32, h: f32) -> Welford<f32> {
    w.update(y, n, h)
}

/// The fixture: the `diffusion` golden scene with each sample's `t_end_step`, `mean_y` and `C_ty` those of its
/// series streamed through `update` at the scene's `dt_macro`.
fn fixture(update: Update) -> Scene {
    let mut s = scene("diffusion").unwrap_or_else(|e| panic!("{e}"));
    let h = s.context.dt_macro;
    for (i, (&n, &(y0, b))) in N.iter().zip(&SERIES).enumerate() {
        let w = (1..=n).fold(Welford::default(), |w, k| {
            update(w, y0 + b * (k as f32 * h), k, h)
        });
        s.set
            .sample(i as u32)
            .times(n, 0)
            .mean_y(w.mean_y)
            .C_ty(w.c_ty);
    }
    s.colouring = Colouring::View("diffusion");
    s
}

/// Checks that, through the read side, `n < 2` reads NaN and is invalid, and each `n ≥ 2` is valid and reads its
/// series' slope `b` within `10⁻⁴` relative, f32 rounding: the kernel's stream is within `1.3·10⁻⁵` of every slope
/// here, and the post-insertion deviation off by a share of the slope (the control).
fn check_reads(s: &Scene) {
    for (i, (&n, &(_, b))) in N.iter().zip(&SERIES).enumerate() {
        let r = s.read(i as u32);
        if n < 2 {
            assert!(!r.diffusion_slope_valid, "n = {n} is valid");
            assert_eq!(
                r.diffusion.to_bits(),
                QNAN,
                "n = {n} reads {}, not NaN",
                r.diffusion
            );
        } else {
            assert!(r.diffusion_slope_valid, "n = {n} is invalid");
            assert!(
                (r.diffusion - b).abs() <= 1e-4 * b.abs(),
                "n = {n} reads {}, not its series' slope {b}",
                r.diffusion
            );
        }
    }
}

#[test]
fn diffusion_sentinel_streaming_reads_the_slope() {
    check_reads(&fixture(kernel_update));
}

#[cfg(feature = "controls")]
/// The update with the post-insertion time deviation `t_n − mean_t(n) = (n − 1)·h/2`, which payload §4 rules out.
fn post_insertion(w: Welford<f32>, y: f32, n: u32, h: f32) -> Welford<f32> {
    let mean_y = w.mean_y + (y - w.mean_y) / n as f32;
    Welford {
        mean_y,
        c_ty: w.c_ty + (n as f32 - 1.0) * h / 2.0 * (y - mean_y),
    }
}

negative_control!(
    diffusion_sentinel_streaming_reads_the_slope,
    "co-moments streamed with the post-insertion time deviation read a slope not their series'",
    expected = "not its series' slope",
    check_reads(&fixture(post_insertion))
);

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

/// `s` rendered, linear RGB.
fn render(h: &GpuHarness, s: &Scene) -> Vec<Rgb> {
    s.render(h.device(), h.queue())
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .map(|p| [p[0], p[1], p[2]].map(f64::from))
        .collect()
}

/// The largest per-channel difference, linear RGB, between a render and its CPU twin, as `numeric_views.rs` has it:
/// the renders sit within 6e-7 of their twins on Metal.
const TOL: f64 = 1e-5;

fn near(a: Rgb, b: Rgb) -> bool {
    a.iter().zip(&b).all(|(x, y)| (x - y).abs() <= TOL)
}

/// Checks the render `image` of `s`: samples with `n < 2` hatched, every other on its ramp, and every pixel its CPU
/// twin's.
fn check_render(s: &Scene, image: &[Rgb]) {
    let width = s.size().0;
    for (i, &n) in N.iter().enumerate() {
        let look = s.look(i as u32).unwrap_or_else(|e| panic!("{e}"));
        if n < 2 {
            assert_eq!(look, Look::Invalid, "n = {n}'s look");
        } else {
            assert!(
                matches!(look, Look::Ramp { .. }),
                "n = {n}'s look is {look:?}"
            );
        }
        for (x, y) in s.pixels(i as u32) {
            let got = image[(y * width + x) as usize];
            let want = look.colour([x, y]);
            assert!(
                near(got, want),
                "pixel ({x}, {y}) of n = {n} is {got:?}, not its look's {want:?}"
            );
        }
    }
}

#[test]
fn diffusion_sentinel_streaming_renders_the_hatch() {
    let s = fixture(kernel_update);
    check_render(&s, &render(&gpu(), &s));
}

negative_control!(
    diffusion_sentinel_streaming_renders_the_hatch,
    "a view drawing n = 1 on the ramp, as a divided slope would place it, is not the hatch",
    expected = "not its look's",
    {
        let s = fixture(kernel_update);
        let mut image = render(&gpu(), &s);
        let width = s.size().0;
        let wrong = Look::Ramp {
            ramp: ledger::gen::numeric::Ramp::Coolwarm,
            t: 0.5,
        };
        for (x, y) in s.pixels(1) {
            image[(y * width + x) as usize] = wrong.colour([x, y]);
        }
        check_render(&s, &image);
    }
);
