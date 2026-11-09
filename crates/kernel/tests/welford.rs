//! The Welford streaming diffusion (`kernel::driver::welford`; dd_simstate_payload §4; dd_integrator §3.5; R-17,
//! R-245), and the wrapper's use of it (`kernel::driver::wrapper`):
//! - REQ-INT-041 (`welford_slope_*`): the streaming slope, the f32 co-moments `mean_y` and `C_ty` read through the
//!   ledger's `diffusion_slope`, equals the least-squares slope of `y` on `t` for random series within an f32
//!   tolerance, and the f64 stream equals it to round-off; `n = 0` and `n = 1` read NaN and are invalid, `n = 2` the
//!   first valid fit; the update reads no shared mutable state, so samples updated interleaved, in any order, are bit
//!   for bit the samples updated alone; and the wrapper streams payload §4's `y`, its `mean_y` and slope those of the
//!   least-squares fit of `y` over the completed macro-steps, a macro-step cut short by `done` taking no sample.
//! - REQ-PAY-054 (`time_moments_closed_form_*`): `C_tt(n) = h²·n(n² − 1)/12` equals the brute-force `Σ(tᵢ − mean_t)²`,
//!   and `mean_t = (n + 1)·h/2` the brute-force mean, for `n = 0 … 1000`; `δ_t = 0.5·n·h` is `t_n − mean_t(n − 1)`; and
//!   the kernel's f32 `C_tt` is the read side's to the bit for every u16 `n`.
//! - REQ-PAY-073 (`welford_y_*`): `y` is the squared shape-sphere chord to the IC's shape: 0 at the IC, 4 at the
//!   antipode, in `[0, 4]`, and the function of `(r, m)` through `kernel::shape::shape`.
//!
//! Each test registers its negative control (R-176).

use kernel::decode::Ic;
use kernel::driver::advance::Cadence;
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::substep::NsubThresholds;
use kernel::driver::welford::{self, c_tt, chord2, delta_t, mean_t, Welford};
use kernel::driver::wrapper::{integrate, integrate_with, WrapperReal, WrapperUniforms};
use kernel::generated::nsub_thresholds::NSUB_TABLE_ROWS;
use kernel::occupant::kdk::Kdk;
use kernel::occupant::{Geom, State};
use kernel::payload::{
    diffusion_c_tt, diffusion_slope, diffusion_slope_valid, tm_t_end_step, SimStateBaseOf,
};
use kernel::shape::shape;
use proptest::prelude::*;
use spirv_std::glam::UVec4;
use validation::{negative_control, prop};

/// The canonical quiet NaN an invalid fit reads (R-245; lowering Part 3a).
const QNAN: u32 = 0x7fc0_0000;

/// The f32 `x` at `R`, exactly.
fn of<R: From<f32>>(x: f32) -> R {
    <R as From<f32>>::from(x)
}

/// A uniform `[0, 1)` stream from `seed` (SplitMix64).
fn stream(seed: u64) -> impl FnMut() -> f64 {
    let mut state = seed;
    move || {
        state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        ((z ^ (z >> 31)) >> 11) as f64 / (1u64 << 53) as f64
    }
}

// ── REQ-INT-041: the streaming slope ────────────────────────────────────────────────────────────────────────────────

/// One series: the step `h` and the samples `y₁ … y_n` at `tᵢ = i·h`, each in `[0, 4]`, the range of payload §4's `y`.
#[derive(Clone, Debug)]
struct Series {
    h: f32,
    y: Vec<f32>,
}

/// A random series from `seed`: `n` in `[2, 65535]` (a u16 horizon), `h` in `[10⁻³, 10⁻¹]`, and `y` a drift, an
/// oscillation and noise, clamped to `[0, 4]`, of random weights.
fn random_series(seed: u64) -> Series {
    let mut next = stream(seed);
    let n = 2 + (next() * 65534.0) as usize;
    let h = 10f64.powf(-3.0 + 2.0 * next()) as f32;
    let (y0, drift, amp, omega, phase, noise) = (
        4.0 * next(),
        (next() - 0.5) * 8.0 / (n as f64 * f64::from(h)),
        2.0 * next(),
        next() * 200.0,
        6.3 * next(),
        2.0 * next(),
    );
    let y = (1..=n)
        .map(|i| {
            let t = i as f64 * f64::from(h);
            let v = y0 + drift * t + amp * (omega * t + phase).sin() + noise * (next() - 0.5);
            v.clamp(0.0, 4.0) as f32
        })
        .collect();
    Series { h, y }
}

/// The least-squares slope of `ys` on `t` in f64, two-pass, over `tᵢ = i·h`, `i = 1 … n`, and the slope's scale for a
/// stream's rounding, `√(Σyᵢ²/Σ(tᵢ − mean_t)²)`: f32 holds each `y` to its own magnitude, not to its deviation.
fn least_squares(h: f64, ys: &[f64]) -> (f64, f64) {
    let n = ys.len() as f64;
    let t = |i: usize| (i + 1) as f64 * h;
    let mt = (0..ys.len()).map(t).sum::<f64>() / n;
    let my = ys.iter().sum::<f64>() / n;
    let (mut ctt, mut cty, mut yy) = (0.0, 0.0, 0.0);
    for (i, &y) in ys.iter().enumerate() {
        let (dt, dy) = (t(i) - mt, y - my);
        ctt += dt * dt;
        cty += dt * dy;
        yy += y * y;
    }
    (cty / ctt, (yy / ctt).sqrt())
}

/// An update rule: the kernel's ([`Welford::update`]) or a control's faulty one.
type Update<R> = fn(Welford<R>, R, u32, R) -> Welford<R>;

fn kernel_update<R: WrapperReal>(w: Welford<R>, y: R, n: u32, h: R) -> Welford<R> {
    w.update(y, n, h)
}

/// The co-moments after streaming `s` through `update` at `R`.
fn streamed<R: WrapperReal + From<f32>>(s: &Series, update: Update<R>) -> Welford<R> {
    s.y.iter()
        .enumerate()
        .fold(Welford::default(), |w, (i, &y)| {
            update(w, of::<R>(y), i as u32 + 1, of::<R>(s.h))
        })
}

/// The f32 tolerance on a slope streamed over `n` samples, as a fraction of its scale ([`least_squares`]): `n·ε`,
/// `ε = 2⁻²³` (f32's epsilon), the first-order forward-error bound of an `n`-step recurrence such as the running
/// `mean_y` and `C_ty`. The largest error measured, over 600 series of up to 65535 samples, was 0.045 of it.
fn slope_tol(n: u32) -> f64 {
    f64::from(n) * f64::from(f32::EPSILON)
}

/// Checks that `update`'s f32 stream of `s`, read through the ledger's `diffusion_slope` at `n = len(s)`, is the
/// least-squares slope within [`slope_tol`] of its scale, and that the f64 stream is it within `10⁻⁹`.
fn check_slope(s: &Series, update32: Update<f32>, update64: Update<f64>) {
    let n = s.y.len() as u32;
    let ys: Vec<f64> = s.y.iter().map(|&y| f64::from(y)).collect();
    let (want, scale) = least_squares(f64::from(s.h), &ys);
    let w = streamed(s, update32);
    let got = diffusion_slope(w.c_ty, n, s.h);
    assert!(diffusion_slope_valid(n), "n = {n} is not valid");
    let err = (f64::from(got) - want).abs();
    assert!(
        err <= slope_tol(n) * scale,
        "the f32 streaming slope {got} differs from the least-squares slope {want} by {err:e}, over n·ε of its scale \
         {scale} (n = {n}, h = {})",
        s.h
    );
    let w = streamed(s, update64);
    let got = w.c_ty / c_tt(n, f64::from(s.h));
    let err = (got - want).abs();
    assert!(
        err <= 1e-9 * scale,
        "the f64 streaming slope {got} differs from the least-squares slope {want} by {err:e}, over 10⁻⁹ of its scale"
    );
}

#[test]
fn welford_slope_matches_least_squares() {
    // A series that clamps at 0 or 4 throughout, `y` constant: slope 0, which the stream reads exactly.
    check_slope(
        &Series {
            h: 0.01,
            y: vec![4.0; 3000],
        },
        kernel_update,
        kernel_update,
    );
    prop::run(&any::<u64>(), |seed| {
        check_slope(&random_series(seed), kernel_update, kernel_update);
        Ok(())
    });
}

#[cfg(feature = "controls")]
/// The update with the post-insertion time deviation `t_n − mean_t(n) = (n − 1)·h/2`, which payload §4 rules out.
fn post_insertion<R: WrapperReal + From<f32>>(w: Welford<R>, y: R, n: u32, h: R) -> Welford<R> {
    let one = of::<R>(1.0);
    let mean_y = w.mean_y + (y - w.mean_y) / of::<R>(n as f32);
    let dt = (of::<R>(n as f32) - one) * h / (one + one);
    Welford {
        mean_y,
        c_ty: w.c_ty + dt * (y - mean_y),
    }
}

negative_control!(
    welford_slope_matches_least_squares,
    "the post-insertion time deviation t_n − mean_t(n), which payload §4 rules out, misses the least-squares slope",
    expected = "differs from the least-squares slope",
    check_slope(
        &Series {
            h: 0.01,
            y: (1..=400).map(|i| (i as f32 * 0.01).min(4.0)).collect(),
        },
        post_insertion,
        post_insertion,
    )
);

/// Checks the sentinel: `n = 0` and `n = 1` read the canonical NaN and are invalid, whatever `C_ty`, and `n = 2`, the
/// first fit, is valid and reads `C_ty / C_tt(2)`, `C_tt(2) = h²/2`.
fn check_sentinel(slope: fn(f32, u32, f32) -> f32) {
    let h = 0.01;
    // One sample: C_ty stays 0; the fit is invalid at n = 0 and n = 1.
    let w = Welford::default().update(1.5, 1, h);
    assert_eq!(w.c_ty, 0.0, "one sample leaves C_ty 0");
    for (n, c_ty) in [(0, 0.0), (1, w.c_ty), (1, 0.25)] {
        assert!(!diffusion_slope_valid(n), "n = {n} is valid");
        let got = slope(c_ty, n, h);
        assert_eq!(got.to_bits(), QNAN, "n = {n} reads {got}, not NaN");
    }
    let w = w.update(2.5, 2, h);
    assert!(diffusion_slope_valid(2), "n = 2 is invalid");
    let got = slope(w.c_ty, 2, h);
    assert!(
        (got - 1.0 / h).abs() <= 1e-3 / h,
        "n = 2's slope {got} is not (2.5 − 1.5)/h"
    );
}

#[test]
fn welford_slope_sentinel_below_two() {
    check_sentinel(diffusion_slope);
}

negative_control!(
    welford_slope_sentinel_below_two,
    "a slope that divides at n = 1, as a −1 sentinel or none would let it, reads no NaN",
    expected = "not NaN",
    check_sentinel(|c_ty, n, h| {
        if n == 0 {
            f32::from_bits(QNAN)
        } else {
            c_ty / diffusion_c_tt(n.max(2), h)
        }
    })
);

/// Checks that `samples` series updated interleaved, round-robin in the order `order` gives, are bit for bit the
/// series each streamed alone: the update reads and writes nothing but its own sample's co-moments and `(y, n, h)`,
/// and the time moments are functions of `(n, h)` only, so no state is shared between samples (REQ-INT-041).
fn check_no_shared_state(series: &[Series], order: &[usize], update: Update<f32>) {
    let alone: Vec<Welford<f32>> = series.iter().map(|s| streamed(s, update)).collect();
    let mut live = vec![Welford::default(); series.len()];
    let steps = series.iter().map(|s| s.y.len()).max().unwrap_or(0);
    for k in 0..steps {
        for &j in order {
            if let Some(&y) = series[j].y.get(k) {
                live[j] = update(live[j], y, k as u32 + 1, series[j].h);
            }
        }
    }
    for (j, (a, b)) in alone.iter().zip(&live).enumerate() {
        assert_eq!(
            (a.mean_y.to_bits(), a.c_ty.to_bits()),
            (b.mean_y.to_bits(), b.c_ty.to_bits()),
            "sample {j} updated interleaved differs from it alone"
        );
    }
}

#[test]
fn welford_slope_no_shared_global() {
    let series: Vec<Series> = (0..4)
        .map(|k| {
            let mut s = random_series(17 + k);
            s.y.truncate(500 + 37 * k as usize);
            s
        })
        .collect();
    check_no_shared_state(&series, &[0, 1, 2, 3], kernel_update);
    check_no_shared_state(&series, &[3, 1, 0, 2], kernel_update);
}

#[cfg(feature = "controls")]
thread_local! {
    /// A control's shared mutable time mean, the global payload §4 rules out.
    static SHARED_MEAN_T: std::cell::Cell<f32> = const { std::cell::Cell::new(0.0) };
}

#[cfg(feature = "controls")]
/// The update with a shared `mean_t`, advanced by every sample's update as an earlier draft's broadcast had it.
fn shared_global(w: Welford<f32>, y: f32, n: u32, h: f32) -> Welford<f32> {
    let dt = SHARED_MEAN_T.with(|m| {
        let old = m.get();
        m.set(old + (n as f32 * h - old) / n as f32);
        n as f32 * h - old
    });
    let mean_y = w.mean_y + (y - w.mean_y) / n as f32;
    Welford {
        mean_y,
        c_ty: w.c_ty + dt * (y - mean_y),
    }
}

negative_control!(
    welford_slope_no_shared_global,
    "a shared mutable mean_t, updated by every sample, makes a sample depend on the others",
    expected = "updated interleaved differs from it alone",
    {
        let series: Vec<Series> = (0..4)
            .map(|k| {
                let mut s = random_series(17 + k);
                s.y.truncate(500 + 37 * k as usize);
                s
            })
            .collect();
        SHARED_MEAN_T.with(|m| m.set(0.0));
        check_no_shared_state(&series, &[0, 1, 2, 3], shared_global);
    }
);

// ── The wrapper streams payload §4's y ──────────────────────────────────────────────────────────────────────────────

/// A sim key with `N_sub = 1` throughout (every threshold 0), `dt_macro` and `horizon_steps` as given.
fn uniforms<R: WrapperReal + From<f32>>(dt_macro: f32, horizon_steps: u32) -> WrapperUniforms<R> {
    WrapperUniforms {
        dt_macro: of::<R>(dt_macro),
        horizon_steps,
        n_max: 1,
        thresholds: NsubThresholds {
            rows: [UVec4::splat(0f32.to_bits()); NSUB_TABLE_ROWS],
        },
    }
}

/// A scalene IC with unequal masses, at rest in its centre-of-mass frame, `Σpᵢ = 0`: a bound, non-collinear start
/// whose shape moves over a few hundred macro-steps.
fn scalene<R: WrapperReal + From<f32>>() -> Ic<R> {
    let f = of::<R>;
    Ic {
        m: [f(0.3), f(0.33), f(0.37)],
        r: [[f(-0.6), f(-0.2)], [f(0.5), f(-0.35)], [f(0.04), f(0.5)]],
        p: [[f(0.05), f(-0.12)], [f(0.08), f(0.1)], [f(-0.13), f(0.02)]],
    }
}

/// The `y` of the wrapper's state after `k` completed macro-steps: the state a `k`-step horizon returns, which the
/// march reaches identically on the way to a longer one.
fn y_after<R: WrapperReal + From<f32> + Into<f64>>(ic: &Ic<R>, dt: f32, k: u32) -> f64 {
    let s = integrate::<Kdk, R>(ic, &uniforms(dt, k));
    welford::y(s.r, ic.m, shape(ic.r, ic.m)).into()
}

/// Checks that the wrapper's `mean_y` and slope after `n` macro-steps are the mean and the least-squares slope of
/// `y_k = ‖n̂(r_k, m) − n̂(r_0, m)‖²` over `k = 1 … n`, each taken from the state a `k`-step run returns, within `tol`
/// of the slope's scale; `out` is the run's `SimState`.
fn check_wrapper_streams_y<R: WrapperReal + From<f32> + Into<f64>>(
    out: &SimStateBaseOf<R>,
    ys: &[f64],
    h: f32,
    tol: f64,
) {
    let n = tm_t_end_step(out.times);
    assert_eq!(n as usize, ys.len(), "t_end_step is not the samples taken");
    let (want, scale) = least_squares(f64::from(h), ys);
    let mean = ys.iter().sum::<f64>() / ys.len() as f64;
    let mean_y: f64 = out.mean_y.into();
    assert!(
        (mean_y - mean).abs() <= tol * 4.0,
        "the wrapper's mean_y {mean_y} is not the mean {mean} of y over the completed macro-steps"
    );
    let c_ty: f64 = out.C_ty.into();
    let got = c_ty / c_tt(n, f64::from(h));
    assert!(
        (got - want).abs() <= tol * scale,
        "the wrapper's slope {got} is not the least-squares slope {want} of y (scale {scale})"
    );
}

#[test]
fn welford_slope_wrapper_streams_y() {
    let (dt, n) = (0.01, 200);
    let ic = scalene::<f64>();
    let ys: Vec<f64> = (1..=n).map(|k| y_after(&ic, dt, k)).collect();
    assert!(
        ys.iter().any(|&y| y > 1e-3),
        "the shape does not move, so the test shows nothing"
    );
    check_wrapper_streams_y(&integrate::<Kdk, f64>(&ic, &uniforms(dt, n)), &ys, dt, 1e-9);
    let ic = scalene::<f32>();
    let ys: Vec<f64> = (1..=n).map(|k| y_after(&ic, dt, k)).collect();
    check_wrapper_streams_y(&integrate::<Kdk, f32>(&ic, &uniforms(dt, n)), &ys, dt, 1e-3);
}

negative_control!(
    welford_slope_wrapper_streams_y,
    "a y taken against the IC's shape at the wrong sign (its antipode) misses the wrapper's mean_y",
    expected = "is not the mean",
    {
        let (dt, n) = (0.01, 200);
        let ic = scalene::<f64>();
        let ys: Vec<f64> = (1..=n)
            .map(|k| {
                let s = integrate::<Kdk, f64>(&ic, &uniforms(dt, k));
                chord2(shape(s.r, ic.m), shape(ic.r, ic.m).map(|x| -x))
            })
            .collect();
        check_wrapper_streams_y(&integrate::<Kdk, f64>(&ic, &uniforms(dt, n)), &ys, dt, 1e-9);
    }
);

/// The wrapper's cadence, but `done` after `limit` substeps: a terminal fired part-way through a macro-step.
struct StopAfter<R> {
    inner: WrapperCadence<R>,
    limit: u32,
}

impl<R: WrapperReal> Cadence<R> for StopAfter<R> {
    fn after_step(&mut self, state: &mut State<R>, geom: &mut Geom<R>) {
        self.inner.after_step(state, geom);
    }

    fn done(&self) -> bool {
        self.inner.invocations() >= self.limit
    }
}

/// Checks that a run `done` stops part-way through macro-step `stop + 1` (`N_sub = 4`) takes samples for its `stop`
/// completed macro-steps only: its `mean_y` and `C_ty` are a `stop`-step horizon's, bit for bit.
fn check_cut_short(skip_cut: bool) {
    let mut key = uniforms::<f64>(0.01, 40);
    key.n_max = 4;
    key.thresholds = NsubThresholds {
        rows: [UVec4::splat(f32::MAX.to_bits()); NSUB_TABLE_ROWS],
    };
    let ic = scalene::<f64>();
    let stop = 17;
    let mut cadence = StopAfter {
        inner: WrapperCadence::new(&ic.m, &ic.r, &ic.p),
        limit: 4 * stop + 2,
    };
    let cut = integrate_with::<Kdk, f64, _>(&ic, &key, &mut cadence);
    let mut whole = key;
    whole.horizon_steps = stop + u32::from(!skip_cut);
    let whole = integrate::<Kdk, f64>(&ic, &whole);
    assert_eq!(tm_t_end_step(cut.times), stop, "the cut macro-step counted");
    assert_eq!(
        (cut.mean_y.to_bits(), cut.C_ty.to_bits()),
        (whole.mean_y.to_bits(), whole.C_ty.to_bits()),
        "the run cut short took a sample its completed macro-steps did not"
    );
}

#[test]
fn welford_slope_cut_short_step_takes_no_sample() {
    check_cut_short(true);
}

negative_control!(
    welford_slope_cut_short_step_takes_no_sample,
    "the co-moments of a horizon one macro-step longer, which a sample of the cut step would give, differ",
    expected = "took a sample its completed macro-steps did not",
    check_cut_short(false)
);

// ── REQ-PAY-054: the time moments in closed form ────────────────────────────────────────────────────────────────────

/// Checks the closed forms against brute force over `tᵢ = i·h`, `i = 1 … n`, for `n = 0 … 1000`: `c_tt` is
/// `Σ(tᵢ − mean)²` within `10⁻¹²` relative (0 for `n < 2`), `mean_t` the mean for `n ≥ 1`, and `delta_t(n)` is
/// `t_n − mean_t(n − 1)`.
fn check_closed_form(c_tt: fn(u32, f64) -> f64) {
    let h = 0.0137;
    for n in 0u32..=1000 {
        let t = |i: u32| f64::from(i) * h;
        let mean = (1..=n).map(t).sum::<f64>() / f64::from(n.max(1));
        let brute = (1..=n).map(|i| (t(i) - mean).powi(2)).sum::<f64>();
        let closed = c_tt(n, h);
        assert!(
            (closed - brute).abs() <= 1e-12 * brute.max(h * h),
            "C_tt({n}) = {closed} is not the brute-force Σ(tᵢ − mean_t)² = {brute}"
        );
        if n >= 1 {
            assert!(
                (mean_t(n, h) - mean).abs() <= 1e-12 * mean,
                "mean_t({n}) = {} is not the brute-force mean {mean}",
                mean_t(n, h)
            );
            assert!(
                (delta_t(n, h) - (t(n) - mean_t(n - 1, h))).abs() <= 1e-12 * t(n),
                "δ_t({n}) is not t_n − mean_t(n − 1)"
            );
        }
    }
    assert_eq!(c_tt(0, h), 0.0, "C_tt(0) is not 0");
    assert_eq!(c_tt(1, h), 0.0, "C_tt(1) is not 0");
}

#[test]
fn time_moments_closed_form_matches_brute_force() {
    check_closed_form(c_tt::<f64>);
}

negative_control!(
    time_moments_closed_form_matches_brute_force,
    "C_tt with n² in place of n² − 1 is not the brute-force sum",
    expected = "is not the brute-force",
    check_closed_form(|n, h| {
        let m = f64::from(n);
        h * h * m * m * m / 12.0
    })
);

/// Checks that the kernel's f32 `C_tt` is `c_tt_read`'s to the bit for every u16 `n`.
fn check_read_side_bits(c_tt_read: fn(u32, f32) -> f32) {
    for h in [1e-3f32, 0.0137, 0.1] {
        for n in 0u32..=65535 {
            assert_eq!(
                c_tt::<f32>(n, h).to_bits(),
                c_tt_read(n, h).to_bits(),
                "C_tt({n}) at h = {h}: the kernel's differs from the read side's"
            );
        }
    }
}

#[test]
fn time_moments_closed_form_kernel_is_the_read_side() {
    check_read_side_bits(diffusion_c_tt);
}

negative_control!(
    time_moments_closed_form_kernel_is_the_read_side,
    "a read side computing C_tt in another order differs in its last bit",
    expected = "the kernel's differs from the read side's",
    check_read_side_bits(|n, h| {
        let m = n as f32;
        (m * m - 1.0) * m * h * h / 12.0
    })
);

// ── REQ-PAY-073: y ──────────────────────────────────────────────────────────────────────────────────────────────────

/// A `y`: payload §4's ([`welford::y`]) or a control's faulty one, of the positions, the masses and the IC's shape.
type YFn = fn([[f64; 2]; 3], [f64; 3], [f64; 3]) -> f64;

/// Checks `y`: 0 at the IC's own shape, 4 at its antipode, in `[0, 4]` over random configurations and masses, and
/// `‖n̂(r, m) − n̂₀‖²` through the shape map.
fn check_y(y: YFn) {
    let ic = scalene::<f64>();
    let n_0 = shape(ic.r, ic.m);
    assert_eq!(y(ic.r, ic.m, n_0), 0.0, "y at the IC is not 0");
    // The antipode, checked on the shape point itself.
    let anti = n_0.map(|x| -x);
    assert!(
        (chord2(n_0, anti) - 4.0).abs() <= 1e-12,
        "the antipode is not at 4"
    );
    let mut next = stream(99);
    for _ in 0..2000 {
        let r = [[0f64; 2]; 3].map(|b| b.map(|_| 4.0 * next() - 2.0));
        let w = [next() + 0.05, next() + 0.05, next() + 0.05];
        let total: f64 = w.iter().sum();
        let m = w.map(|x| x / total);
        let v = y(r, m, n_0);
        let want = {
            let n = shape(r, m);
            (0..3).map(|i| (n[i] - n_0[i]).powi(2)).sum::<f64>()
        };
        assert!(
            (-1e-12..=4.0 + 1e-12).contains(&v),
            "y = {v} is outside [0, 4]"
        );
        assert!(
            (v - want).abs() <= 1e-12,
            "y = {v} is not ‖n̂(r, m) − n̂₀‖² = {want}"
        );
    }
}

#[test]
fn welford_y_is_the_shape_chord() {
    check_y(welford::y::<f64>);
}

negative_control!(
    welford_y_is_the_shape_chord,
    "the chord, not its square, is not payload §4's y",
    expected = "is not ‖n̂(r, m) − n̂₀‖²",
    check_y(|r, m, n_0| welford::y(r, m, n_0).sqrt())
);
