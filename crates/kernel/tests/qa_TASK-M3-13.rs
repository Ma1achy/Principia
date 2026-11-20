//! QA's tests for TASK-M3-13, written from the requirements and their sources (dd_simstate_payload §4; dd_integrator
//! §3.5, §3.7; R-17, R-72, R-245), not from the implementation:
//! - REQ-INT-041 (`qa_m3_13_welford_slope_*`): the f32 streaming co-moments, read through the read side's
//!   `diffusion_slope`, give the least-squares slope of `y` on `t` within an f32 tolerance that is a first-order
//!   running round-off bound on the update's own operations (Higham, *Accuracy and Stability of Numerical Algorithms*,
//!   §3.3), never a number picked; `n = 0` and `n = 1` read the canonical NaN (R-245), whatever `C_ty` holds; samples
//!   streamed concurrently on threads in lockstep, as one dispatch's invocations are, are bit for bit the samples
//!   streamed alone, so no shared mutable state enters the update.
//! - REQ-PAY-054 (`qa_m3_13_time_moments_*`): `C_tt(n) = h²·n(n²−1)/12` and `mean_t = (n+1)h/2` are exactly the
//!   brute-force `Σ(tᵢ − mean_t)²` and mean over `tᵢ = i·h` for `n = 0 … 1000` at dyadic `h`, where both are exact in
//!   f64; at f32 within the closed form's five roundings; `δ_t = 0.5·n·h` is `t_n − mean_t(n − 1)`.
//! - REQ-PAY-073 (`qa_m3_13_y_*`): payload §4's `y = ‖n̂(r, m) − n̂₀‖²` at the shape sphere's landmarks, which
//!   dd_integrator §3.7 places independently of the code: `L⁺ = (0, 0, 1)` for the anticlockwise equilateral triangle at
//!   equal masses, `L⁻` its mirror, `b̂₀₁ = (−1, 0, 0)` for bodies 0 and 1 coincident at any masses, and every collinear
//!   configuration on the equator `w = 0` at any masses; `y` is invariant under rotation, scale and translation (a
//!   shape quantity), and in `[0, 4]`.
//! - REQ-PAY-098, kernel side (`qa_m3_13_wrapper_mean_y_in_declared_range`): the wrapper's `mean_y` over real
//!   integrations lies in the declared `[0, 4]`, and the read side's slope of the same run is valid and finite.
//!
//! **Domain** (pitfalls §10): the update and the time moments are stateless functions of their arguments; the wrapper
//! test runs real trajectories, which is the regime the range claim is about.
//!
//! Each test has its negative control (R-176).

use std::sync::Barrier;

use kernel::decode::Ic;
use kernel::driver::substep::NsubThresholds;
use kernel::driver::welford::{self, c_tt, delta_t, mean_t, Welford};
use kernel::driver::wrapper::{integrate, WrapperUniforms};
use kernel::generated::nsub_thresholds::NSUB_TABLE_ROWS;
use kernel::occupant::kdk::Kdk;
use kernel::payload::{diffusion_slope, diffusion_slope_valid, tm_t_end_step};
use spirv_std::glam::UVec4;
use validation::negative_control;

/// The canonical quiet NaN (R-245; lowering Part 3a).
const QNAN: u32 = 0x7fc0_0000;

/// f32's unit roundoff, `2⁻²⁴`.
const U32: f64 = f32::EPSILON as f64 / 2.0;

/// A uniform `[0, 1)` stream (SplitMix64), the test's own.
fn rng(seed: u64) -> impl FnMut() -> f64 {
    let mut s = seed ^ 0x5151_a5a5_0f0f_3c3c;
    move || {
        s = s.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = s;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        ((z ^ (z >> 31)) >> 11) as f64 / (1u64 << 53) as f64
    }
}

// ── REQ-INT-041: the streaming slope ────────────────────────────────────────────────────────────────────────────────

/// A series: the macro-step `h` and the samples `y₁ … y_n` at `tᵢ = i·h`, each in `[0, 4]` (payload §4's range of `y`).
struct Series {
    h: f32,
    y: Vec<f32>,
}

/// Series of four kinds (a linear trend with noise, a reflected random walk, i.i.d. uniform, a constant) at each of
/// the lengths and macro-steps below, `n ≥ 2` throughout.
fn series() -> Vec<Series> {
    let mut next = rng(1313);
    let mut out = Vec::new();
    for &n in &[2usize, 3, 4, 7, 16, 64, 255, 1000] {
        for &h in &[0.01f32, 0.001, 0.05] {
            for kind in 0..4 {
                let mut y = Vec::with_capacity(n);
                let (a, b) = (
                    4.0 * next(),
                    (next() - 0.5) * 8.0 / (n as f64 * f64::from(h)),
                );
                let mut walk = 4.0 * next();
                let c = 4.0 * next();
                for i in 1..=n {
                    let v = match kind {
                        0 => a + b * i as f64 * f64::from(h) + 0.1 * (next() - 0.5),
                        1 => {
                            walk += 0.2 * (next() - 0.5);
                            walk
                        }
                        2 => 4.0 * next(),
                        _ => c,
                    };
                    y.push(v.clamp(0.0, 4.0) as f32);
                }
                out.push(Series { h, y });
            }
        }
    }
    out
}

/// The least-squares slope of `y` on `t` and its centred co-moment, two-pass in f64 on the f32 samples and times
/// (`tᵢ = i·h` exact in f64), with the f64 reference's own round-off bound on the slope, `4n·ε₆₄·Σ|tᵢ−t̄||yᵢ−ȳ|/C_tt`.
fn least_squares(s: &Series) -> (f64, f64, f64) {
    let n = s.y.len() as f64;
    let h = f64::from(s.h);
    let t: Vec<f64> = (1..=s.y.len()).map(|i| i as f64 * h).collect();
    let tm = t.iter().sum::<f64>() / n;
    let ym = s.y.iter().map(|&v| f64::from(v)).sum::<f64>() / n;
    let (mut sty, mut stt, mut mag) = (0.0, 0.0, 0.0);
    for (ti, &yi) in t.iter().zip(&s.y) {
        let (dt, dy) = (ti - tm, f64::from(yi) - ym);
        sty += dt * dy;
        stt += dt * dt;
        mag += (dt * dy).abs();
    }
    (sty / stt, ym, 4.0 * n * f64::EPSILON * mag / stt)
}

type Update = fn(Welford<f32>, f32, u32, f32) -> Welford<f32>;

fn kernel_update(w: Welford<f32>, y: f32, n: u32, h: f32) -> Welford<f32> {
    w.update(y, n, h)
}

/// Streams `s` through `update` and, alongside, a first-order running bound on the round-off of the update's
/// operations (Higham §3.3): per sample, `d = y − m`, `q = d/n`, `m' = m + q` round with error `u|·|` each, the mean's
/// carried error shrinking by `(1 − 1/n)`; `δ_t = (0.5·n)·h`, `e = y − m'`, `p = δ_t·e`, `c' = c + p` round with
/// `u|·|` each, the mean's error entering through `δ_t·E_m`. Returns the co-moments and the bounds `(E_m, E_c)`; the
/// `O(u²)` terms are below `n·u ≤ 6·10⁻⁵` of these and are covered by the caller's `(1 + 10⁻³)`.
fn stream(s: &Series, update: Update) -> (Welford<f32>, f64, f64) {
    let mut w = Welford::<f32>::default();
    let (mut em, mut ec) = (0.0f64, 0.0f64);
    for (i, &y) in s.y.iter().enumerate() {
        let n = i as u32 + 1;
        let old = w;
        w = update(w, y, n, s.h);
        let d = (f64::from(y) - f64::from(old.mean_y)).abs();
        em = em * (1.0 - 1.0 / f64::from(n))
            + U32 * (2.0 * d / f64::from(n) + f64::from(w.mean_y).abs());
        let dt = 0.5 * f64::from(n) * f64::from(s.h);
        let e = (f64::from(y) - f64::from(w.mean_y)).abs();
        ec += dt * em + U32 * (3.0 * dt * e + f64::from(w.c_ty).abs());
    }
    (w, em, ec)
}

/// Checks every series: the mean within `E_m`; the slope read through the read side within
/// `(E_c/C_tt)(1 + 6u) + 6u|b| + ref`, `6u` the read side's `C_tt` (five roundings) and division, `ref` the f64
/// reference's own bound, all times `1 + 10⁻³` for the second-order terms.
fn check_slope(update: Update) {
    for s in series() {
        let n = s.y.len() as u32;
        let (w, em, ec) = stream(&s, update);
        let (b, ym, refb) = least_squares(&s);
        assert!(
            (f64::from(w.mean_y) - ym).abs()
                <= 1.001 * em + 4.0 * f64::from(n) * f64::EPSILON * 4.0,
            "n = {n}, h = {}: mean_y {} is not the mean {ym} (bound {em:e})",
            s.h,
            w.mean_y
        );
        let got = diffusion_slope(w.c_ty, n, s.h);
        let ctt =
            f64::from(s.h) * f64::from(s.h) * f64::from(n) * (f64::from(n).powi(2) - 1.0) / 12.0;
        let tol = 1.001 * ((ec / ctt) * (1.0 + 6.0 * U32) + 6.0 * U32 * b.abs() + refb);
        assert!(
            (f64::from(got) - b).abs() <= tol,
            "n = {n}, h = {}: the streamed slope {got} is not the least-squares slope {b} (tolerance {tol:e})",
            s.h
        );
    }
}

#[test]
fn qa_m3_13_welford_slope_matches_least_squares() {
    check_slope(kernel_update);
}

#[cfg(feature = "controls")]
/// The update with the post-insertion time deviation `t_n − mean_t(n) = (n − 1)h/2`, which payload §4 rules out.
fn post_insertion(w: Welford<f32>, y: f32, n: u32, h: f32) -> Welford<f32> {
    let mean_y = w.mean_y + (y - w.mean_y) / n as f32;
    Welford {
        mean_y,
        c_ty: w.c_ty + (n as f32 - 1.0) * 0.5 * h * (y - mean_y),
    }
}

negative_control!(
    qa_m3_13_welford_slope_matches_least_squares,
    "the post-insertion time deviation misses the least-squares slope",
    expected = "is not the least-squares slope",
    check_slope(post_insertion)
);

/// Checks that `n = 0` and `n = 1` read the canonical NaN and are invalid for any `C_ty`, through `slope`; that one
/// streamed sample leaves `mean_y = y` and `C_ty = 0`; and that `n = 2` is valid and finite.
fn check_nan_below_two(slope: fn(f32, u32, f32) -> f32) {
    for c in [0.0f32, 1.0, -3.5, 1e-30, 7e20] {
        for n in [0u32, 1] {
            assert!(!diffusion_slope_valid(n), "n = {n} is valid");
            let v = slope(c, n, 0.01);
            assert_eq!(v.to_bits(), QNAN, "n = {n}, C_ty = {c} reads {v}, not NaN");
        }
        assert!(diffusion_slope_valid(2), "n = 2 is invalid");
        assert!(slope(c, 2, 0.01).is_finite(), "n = 2 is not a finite slope");
    }
    let w = Welford::<f32>::default().update(2.75, 1, 0.01);
    assert_eq!((w.mean_y, w.c_ty), (2.75, 0.0), "one sample's co-moments");
}

#[test]
fn qa_m3_13_welford_slope_nan_below_two() {
    check_nan_below_two(diffusion_slope);
}

negative_control!(
    qa_m3_13_welford_slope_nan_below_two,
    "the slope divided with no validity predicate is not NaN at n = 1",
    expected = "not NaN",
    check_nan_below_two(|c, n, h| c / c_tt::<f32>(n, h))
);

/// Streams `k` series concurrently, one thread each, in lockstep on a barrier, each through `update`, and checks each
/// thread's co-moments bit for bit against the same series streamed alone.
fn check_lockstep(update: Update) {
    let all = series();
    let group: Vec<&Series> = all.iter().filter(|s| s.y.len() == 64).take(6).collect();
    let alone: Vec<Welford<f32>> = group.iter().map(|s| stream(s, update).0).collect();
    let barrier = Barrier::new(group.len());
    let together: Vec<Welford<f32>> = std::thread::scope(|sc| {
        let hs: Vec<_> = group
            .iter()
            .map(|s| {
                let barrier = &barrier;
                sc.spawn(move || {
                    let mut w = Welford::<f32>::default();
                    for (i, &y) in s.y.iter().enumerate() {
                        barrier.wait();
                        w = update(w, y, i as u32 + 1, s.h);
                        barrier.wait();
                    }
                    w
                })
            })
            .collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    for (k, (a, b)) in alone.iter().zip(&together).enumerate() {
        assert_eq!(
            (a.mean_y.to_bits(), a.c_ty.to_bits()),
            (b.mean_y.to_bits(), b.c_ty.to_bits()),
            "series {k} streamed beside others differs from it streamed alone (shared state)"
        );
    }
}

#[test]
fn qa_m3_13_welford_slope_no_shared_global() {
    check_lockstep(kernel_update);
}

#[cfg(feature = "controls")]
static SHARED_N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "controls")]
/// The earlier draft payload §4 rules out: one shared step count, advanced by every invocation's update.
fn shared_count(w: Welford<f32>, y: f32, _n: u32, h: f32) -> Welford<f32> {
    let n = SHARED_N.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
    w.update(y, n, h)
}

negative_control!(
    qa_m3_13_welford_slope_no_shared_global,
    "an update reading one step count shared by all invocations differs in lockstep",
    expected = "(shared state)",
    check_lockstep(shared_count)
);

// ── REQ-PAY-054: the time moments in closed form ────────────────────────────────────────────────────────────────────

/// Checks, for `n = 0 … 1000` and the dyadic `h = 1, 1/4, 1/128`, where every term and sum below is exact in f64,
/// that `c_tt(n, h)` is exactly the brute-force `Σ(tᵢ − mean)²`, `mean_t(n, h)` exactly the mean (for `n ≥ 1`), and
/// `delta_t(n, h)` exactly `t_n − mean_t(n − 1)`; and that the f32 `c_tt` is within its five roundings, `5u`
/// relative, of the exact value.
fn check_closed_form(ctt: fn(u32, f64) -> f64) {
    for h in [1.0f64, 0.25, 1.0 / 128.0] {
        for n in 0u32..=1000 {
            let t: Vec<f64> = (1..=n).map(|i| f64::from(i) * h).collect();
            let mean = if n == 0 {
                0.0
            } else {
                t.iter().sum::<f64>() / f64::from(n)
            };
            let brute: f64 = t.iter().map(|ti| (ti - mean).powi(2)).sum();
            assert_eq!(
                ctt(n, h),
                brute,
                "C_tt({n}, {h}) is not the brute-force sum (closed form)"
            );
            if n >= 1 {
                assert_eq!(mean_t(n, h), mean, "mean_t({n}, {h}) is not the mean");
                assert_eq!(
                    delta_t(n, h),
                    f64::from(n) * h - mean_t(n - 1, h),
                    "δ_t({n}, {h}) is not t_n − mean_t(n − 1)"
                );
            }
            let c32 = f64::from(c_tt::<f32>(n, h as f32));
            assert!(
                (c32 - brute).abs() <= 5.0 * U32 * brute,
                "the f32 C_tt({n}, {h}) = {c32} is not within 5u of {brute}"
            );
        }
    }
}

#[test]
fn qa_m3_13_time_moments_closed_form() {
    check_closed_form(c_tt::<f64>);
}

negative_control!(
    qa_m3_13_time_moments_closed_form,
    "C_tt with n(n² + 1) for n(n² − 1) is not the brute-force sum",
    expected = "(closed form)",
    check_closed_form(|n, h| h * h * f64::from(n) * (f64::from(n).powi(2) + 1.0) / 12.0)
);

// ── REQ-PAY-073: y at the shape sphere's landmarks ─────────────────────────────────────────────────────────────────

type YFn = fn([[f64; 2]; 3], [f64; 3], [f64; 3]) -> f64;

/// The tolerance on an f64 `y`: 256 roundings of quantities at most 4, generous against the shape map's few dozen.
const TOL_Y: f64 = 256.0 * f64::EPSILON * 4.0;

const THIRDS: [f64; 3] = [1.0 / 3.0; 3];

/// The equilateral triangle on the unit circle, bodies 0 → 1 → 2 anticlockwise when `sense = 1`, clockwise at −1.
fn equilateral(sense: f64) -> [[f64; 2]; 3] {
    let a = |k: f64| {
        let th = sense * k * 2.0 * std::f64::consts::PI / 3.0;
        [th.cos(), th.sin()]
    };
    [a(0.0), a(1.0), a(2.0)]
}

/// `r` rotated by `phi`, scaled by `s` and translated by `d`.
fn moved(r: [[f64; 2]; 3], phi: f64, s: f64, d: [f64; 2]) -> [[f64; 2]; 3] {
    r.map(|[x, y]| {
        [
            s * (phi.cos() * x - phi.sin() * y) + d[0],
            s * (phi.sin() * x + phi.cos() * y) + d[1],
        ]
    })
}

fn near(got: f64, want: f64, what: &str) {
    assert!((got - want).abs() <= TOL_Y, "{what}: y = {got}, not {want}");
}

/// Checks `y` at the landmarks of dd_integrator §3.7 and its invariances.
fn check_landmarks(y: YFn) {
    let lp = [0.0, 0.0, 1.0];
    let b01 = [-1.0, 0.0, 0.0];
    // L⁺ is the anticlockwise equilateral triangle at equal masses; its mirror is L⁻, the antipode.
    near(y(equilateral(1.0), THIRDS, lp), 0.0, "L⁺ against L⁺");
    near(y(equilateral(-1.0), THIRDS, lp), 4.0, "L⁻ against L⁺");
    let mut next = rng(73);
    for _ in 0..200 {
        let w = [next() + 0.05, next() + 0.05, next() + 0.05];
        let tot: f64 = w.iter().sum();
        let m = w.map(|x| x / tot);
        // A collinear configuration lies on the equator for any masses: its chord² to a pole is 1 + 1 = 2.
        let xs = [4.0 * next() - 2.0, 4.0 * next() - 2.0, 4.0 * next() - 2.0];
        let (dir, off) = (next() * 6.0, [next(), next()]);
        let line = xs.map(|x| [x * dir.cos() + off[0], x * dir.sin() + off[1]]);
        near(y(line, m, lp), 2.0, "a collinear configuration against L⁺");
        // Bodies 0 and 1 coincident is b̂₀₁ = (−1, 0, 0) for any masses.
        let p = [4.0 * next() - 2.0, 4.0 * next() - 2.0];
        let q = [4.0 * next() - 2.0, 4.0 * next() - 2.0];
        near(y([p, p, q], m, b01), 0.0, "the 0-1 collision against b̂₀₁");
        near(
            y([p, p, q], m, [1.0, 0.0, 0.0]),
            4.0,
            "the 0-1 collision against −b̂₀₁",
        );
        // A shape quantity: rotation, scale and translation leave it.
        let r = [[0f64; 2]; 3].map(|b| b.map(|_| 4.0 * next() - 2.0));
        let n0 = {
            let v = [next() - 0.5, next() - 0.5, next() - 0.5];
            let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            v.map(|x| x / l)
        };
        let base = y(r, m, n0);
        assert!(
            (-TOL_Y..=4.0 + TOL_Y).contains(&base),
            "y = {base} is outside [0, 4]"
        );
        let r2 = moved(
            r,
            next() * 6.3,
            0.1 + 5.0 * next(),
            [next() - 0.5, next() - 0.5],
        );
        assert!(
            (y(r2, m, n0) - base).abs() <= 1e3 * TOL_Y,
            "y moved under rotation, scale and translation"
        );
    }
}

#[test]
fn qa_m3_13_y_landmarks() {
    check_landmarks(welford::y::<f64>);
}

negative_control!(
    qa_m3_13_y_landmarks,
    "the chord, unsquared, misses L⁻'s 4 and the equator's 2 against L⁺",
    expected = "against L⁺: y = ",
    check_landmarks(|r, m, n0| welford::y::<f64>(r, m, n0).sqrt())
);

// ── REQ-PAY-098, kernel side: the wrapper's mean_y lies in the declared [0, 4] ──────────────────────────────────────

/// A sim key with `N_sub = 1` throughout, `dt_macro` and `horizon_steps` as given.
fn key(dt: f32, horizon: u32) -> WrapperUniforms<f32> {
    WrapperUniforms {
        dt_macro: dt,
        horizon_steps: horizon,
        n_max: 1,
        thresholds: NsubThresholds {
            rows: [UVec4::splat(0f32.to_bits()); NSUB_TABLE_ROWS],
        },
    }
}

/// Random bound-ish ICs in their centre-of-mass frame, `Σpᵢ = 0`, unequal masses.
fn ics() -> Vec<Ic<f32>> {
    let mut next = rng(98);
    (0..12)
        .map(|_| {
            let w = [next() + 0.2, next() + 0.2, next() + 0.2];
            let tot: f64 = w.iter().sum();
            let m = w.map(|x| (x / tot) as f32);
            let mut r = [[0f32; 2]; 3].map(|b| b.map(|_| (2.0 * next() - 1.0) as f32));
            let mut p = [[0f32; 2]; 3].map(|b| b.map(|_| (0.3 * next() - 0.15) as f32));
            let mt: f32 = m.iter().sum();
            for d in 0..2 {
                let c = (0..3).map(|i| m[i] * r[i][d]).sum::<f32>() / mt;
                let ps = (0..3).map(|i| p[i][d]).sum::<f32>() / 3.0;
                for i in 0..3 {
                    r[i][d] -= c;
                    p[i][d] -= ps;
                }
            }
            Ic { m, r, p }
        })
        .collect()
}

fn check_in_range(mean_y: f32, what: &str) {
    assert!(
        mean_y.is_finite() && (0.0..=4.0).contains(&mean_y),
        "{what}: mean_y = {mean_y} is outside the declared [0, 4]"
    );
}

#[test]
fn qa_m3_13_wrapper_mean_y_in_declared_range() {
    let (dt, horizon) = (0.01f32, 400);
    let mut spread = 0;
    for (k, ic) in ics().iter().enumerate() {
        let out = integrate::<Kdk, f32>(ic, &key(dt, horizon));
        let n = tm_t_end_step(out.times);
        check_in_range(out.mean_y, &format!("IC {k}"));
        spread += usize::from(out.mean_y > 1e-3);
        if n >= 2 {
            let b = diffusion_slope(out.C_ty, n, dt);
            assert!(b.is_finite(), "IC {k}: the slope over n = {n} is {b}");
        }
    }
    assert!(
        spread >= 6,
        "only {spread} of 12 runs moved their shape: the range check shows little"
    );
}

negative_control!(
    qa_m3_13_wrapper_mean_y_in_declared_range,
    "y taken against an IC shape not normalised to the unit sphere (3·n̂₀) leaves [0, 4]",
    expected = "outside the declared [0, 4]",
    {
        let ic = &ics()[0];
        let n0 = kernel::shape::shape(ic.r, ic.m).map(|x| 3.0 * x);
        let mut w = Welford::<f32>::default();
        for k in 1..=40u32 {
            let s = integrate::<Kdk, f32>(ic, &key(0.01, k));
            w = w.update(welford::y(s.r, ic.m, n0), k, 0.01);
        }
        check_in_range(w.mean_y, "control");
    }
);
