//! The decode's `cos` near `π/2` on each backend, against f64, relative to its value.
//!
//! WGSL bounds `sin` and `cos` only by an absolute error (2⁻¹¹ inside `[−π, π]`), but the decode's `‖ρ̃‖ = cos α`
//! (`kernel::decode::config::hyperspherical`) needs a relative one where `α → π/2`: there `cos α` is small, and an
//! error of a fixed size is a large share of it. So the decode calls the kernel's own `cos`, `kernel::trig::sin_cos_f32`,
//! integer arithmetic alone, never the backend's builtin. This file dispatches that function from the kernel's f32
//! build, `target/spirv/kernel.wgsl` (`cargo xtask build-kernel` writes it; the `gpu-kernel` profile), through
//! `kernel::stage::decode::sine_cosine`, under each fast-math setting, on whatever adapter the harness opens
//! (`PRIN_GPU_BACKEND`, or the platform's), and names the adapter in its output. The builtin's own numbers are
//! measured, not held, in `builtin_trig_gpu.rs`.
//!
//! **The arguments.** Every f32 from `π/4` (rounded) up to the decode's largest `α`, the f32 image of `π/2` rounded
//! toward zero (the generated `LIT_6`, `0x3FC90FDA`), about 8.4·10⁶ of them: the half of `α`'s range where
//! `cos α ≤ sin α`, down to the smallest `cos α` the decode can produce, `cos(LIT_6) ≈ 7.55·10⁻⁸`.
//!
//! **The criterion,** written before the first run. For each argument `x`,
//! `|cos_gpu(x) − cos(x as f64)| ≤ δ_max·|cos(x as f64)|`, with `δ_max = ε_phys / 4`, `ε_phys = EPS_PHYS·2⁻²³` the
//! round trip's f32 tolerance: `δ_max = 16·2⁻²³ ≈ 1.91·10⁻⁶`. A NaN fails. And the GPU's `sin` and `cos` are the CPU's
//! `kernel::trig::sin_cos_f32`, bit for bit, under both fast-math settings: the function is integer arithmetic, which no
//! backend may round or reassociate, so the CPU's exhaustive sweep (`crates/kernel/tests/trig.rs`) holds for the GPU.
//! The identity is also held on every 997th f32 of `[−2π, 2π]`, the decode's other arguments.
//! Why a quarter: a relative error `δ` in `cos α` moves `ρ̃`, so `ρ = ρ̃/√μ_ρ`, by `δ‖ρ‖`, and the positions
//! `r₀ = r₀₁ − w₀ρ` and `r₁ = r₀₁ + w₁ρ` by at most `δ‖ρ‖` (`w₀, w₁ ≤ 1`); since `ρ = r₁ − r₀`, `‖ρ‖ ≤ 2R_x`, so the
//! residual `‖Δrᵢ‖/R_x` (`validation::roundtrip`) is at most `2δ`. The plan is unclear here (it says only "the bound
//! read from ε_phys's budget"); chose to give `cos α` half of `ε_phys`, so `2δ ≤ ε_phys/2`, because the rest of the
//! round trip already measures up to 25.64 `ε_R` of the 64 (`ledger::constants::EPS_PHYS`'s note), which the other
//! half covers. The momenta's share is not counted here.
//!
//! **The output.** The worst relative error in each band of `cos x`, `[2⁻⁽ᵏ⁺¹⁾, 2⁻ᵏ)`, in f32 eps (`2⁻²³`), with its
//! argument, printed before the assertion so a failure shows the distribution, not one number.
//!
//! **The controls.** Two stand-in `cos`es, each well inside WGSL's absolute bound of 2⁻¹¹, must fail: the f64 value
//! plus one f32 eps (`2⁻²³`, absolute), on the CPU; and `1 − 2 sin²(x/2)` on the GPU, whose absolute error of about
//! f32's eps near `cos x = 0` is a relative error that grows as `cos x` shrinks (on Metal, an M3 Pro, 55% of `cos x` at
//! `x = LIT_6`). Each control first asserts its stand-in is inside the 2⁻¹¹ absolute bound. A third, the kernel's
//! entry read with its `cos` word one ulp up, must fail the bitwise identity.

use std::path::Path;

use engine::contract::fast_math::FastMath;
use engine::decode_dispatch::sine_cosine_gpu;
use kernel::generated::links::Literals;
use kernel::trig::sin_cos_f32;
use ledger::constants::EPS_PHYS;
use validation::gpu::GpuHarness;
use validation::negative_control;

/// The decode's largest `α`: the f32 image of `π/2`, rounded toward zero.
const TOP: f32 = <f32 as Literals>::LIT_6;

/// The sweep's lowest argument, `π/4` rounded: below it `cos α > sin α`, and `cos α ≥ ½·√2`.
const BOTTOM: f32 = std::f32::consts::FRAC_PI_4;

/// WGSL's absolute bound on `cos` inside `[−π, π]`.
const WGSL_ABSOLUTE: f64 = 1.0 / 2048.0;

/// The arguments one dispatch takes, under a dispatch's 65535 workgroups of 64.
const BATCH: usize = 1 << 20;

/// The bands of `cos x` the report reads: `[2⁻⁽ᵏ⁺¹⁾, 2⁻ᵏ)` for `k` below this.
const BANDS: usize = 26;

/// The WGSL `cargo xtask build-kernel` wrote.
fn wgsl() -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/spirv/kernel.wgsl"),
    )
    .expect("target/spirv/kernel.wgsl: run `cargo xtask build-kernel` first")
}

/// The control's stand-in: `cos x = 1 − 2 sin²(x/2)`, accurate only absolutely.
#[cfg(feature = "controls")]
const HALF_ANGLE_WGSL: &str = r"
@group(0) @binding(0) var<storage, read> x: array<u32>;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;
@compute @workgroup_size(64)
fn half_angle(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x < arrayLength(&x)) {
        let s = sin(0.5 * bitcast<f32>(x[id.x]));
        output[id.x] = bitcast<u32>(1.0 - 2.0 * s * s);
    }
}
";

/// The relative bound `δ_max = ε_phys/4` (the module's note).
fn bound() -> f64 {
    EPS_PHYS.number() * f64::from(f32::EPSILON) / 4.0
}

/// Every f32 in `[BOTTOM, TOP]`.
fn arguments() -> Vec<f32> {
    (BOTTOM.to_bits()..=TOP.to_bits())
        .map(f32::from_bits)
        .collect()
}

/// `(sin, cos)` of each argument by the kernel's entry, under `setting`, in batches of [`BATCH`].
fn kernel_gpu(h: &GpuHarness, wgsl: &str, xs: &[f32], setting: FastMath) -> Vec<(f32, f32)> {
    let mut out = Vec::with_capacity(xs.len());
    for chunk in xs.chunks(BATCH) {
        let got = sine_cosine_gpu(h.device(), h.queue(), wgsl, chunk, setting)
            .expect("the sine_cosine dispatch");
        assert_eq!(got.len(), chunk.len(), "one result per argument");
        out.extend(got);
    }
    out
}

/// `cos` of each argument by the WGSL `module`'s `entry`, under `setting`, in batches of [`BATCH`].
#[cfg(feature = "controls")]
fn gpu(h: &GpuHarness, module: &str, entry: &str, xs: &[f32], setting: FastMath) -> Vec<f32> {
    let mut out = Vec::with_capacity(xs.len());
    for chunk in xs.chunks(BATCH) {
        let words: Vec<u32> = chunk.iter().map(|x| x.to_bits()).collect();
        let got = h.run_wgsl_with(module, entry, &[&words], setting);
        assert_eq!(got.len(), chunk.len(), "{entry}: one result per argument");
        out.extend(got.into_iter().map(f32::from_bits));
    }
    out
}

/// Panics at the first argument whose GPU `(sin, cos)` is not the CPU's `sin_cos_f32`, bit for bit; returns the count.
fn check_identity(label: &str, xs: &[f32], got: &[(f32, f32)]) -> usize {
    assert_eq!(got.len(), xs.len(), "{label}: one result per argument");
    for (&x, &(s, c)) in xs.iter().zip(got) {
        let (ws, wc) = sin_cos_f32(x);
        assert!(
            (s.to_bits(), c.to_bits()) == (ws.to_bits(), wc.to_bits()),
            "{label}: the GPU's sin and cos are not the CPU's at {x:e} ({:#010x}): ({:#010x}, {:#010x}) against \
             ({:#010x}, {:#010x})",
            x.to_bits(),
            s.to_bits(),
            c.to_bits(),
            ws.to_bits(),
            wc.to_bits()
        );
    }
    xs.len()
}

/// `|got − cos x| / |cos x|`, `cos x` at f64; NaN where `got` is.
fn relative(x: f32, got: f32) -> f64 {
    let want = f64::from(x).cos();
    (f64::from(got) - want).abs() / want.abs()
}

/// The largest `|got − cos x|` over the sweep, `cos x` at f64.
fn absolute_worst(xs: &[f32], got: &[f32]) -> f64 {
    xs.iter()
        .zip(got)
        .map(|(&x, &g)| (f64::from(g) - f64::from(x).cos()).abs())
        .fold(0.0, f64::max)
}

/// Prints the worst relative error per band of `cos x`, in f32 eps; returns the arguments past the bound (NaN
/// included), with the first of them.
fn report(label: &str, xs: &[f32], got: &[f32]) -> (usize, Option<(f32, f32, f64)>) {
    let eps = f64::from(f32::EPSILON);
    let mut bands = [(0usize, 0.0f64, 0.0f32); BANDS];
    let (mut over, mut first) = (0, None);
    for (&x, &g) in xs.iter().zip(got) {
        let d = relative(x, g);
        let k = (-f64::from(x).cos().log2()).floor() as usize;
        let band = &mut bands[k.min(BANDS - 1)];
        band.0 += 1;
        if d.is_nan() || d > band.1 {
            (band.1, band.2) = (d, x);
        }
        if d.is_nan() || d > bound() {
            over += 1;
            first.get_or_insert((x, g, d));
        }
    }
    eprintln!("{label}: worst |cos_gpu − cos_f64| / cos_f64 per band of cos x, in f32 eps (bound {:.1} eps)", bound() / eps);
    for (k, &(n, worst, at)) in bands.iter().enumerate().filter(|b| b.1 .0 > 0) {
        eprintln!(
            "  cos x in [2^-{}, 2^-{k}): {n:>8} args, worst {:>12.4} eps at x = {at:e} ({:#010x})",
            k + 1,
            worst / eps,
            at.to_bits()
        );
    }
    eprintln!("{label}: {over} of {} arguments past the bound", xs.len());
    (over, first)
}

/// The criterion (the module's note): every argument's relative error within [`bound`].
fn check(label: &str, xs: &[f32], got: &[f32]) {
    assert_eq!(got.len(), xs.len(), "{label}: one result per argument");
    let (over, first) = report(label, xs, got);
    if let Some((x, g, d)) = first {
        panic!(
            "{label}: {over} arguments' cos exceeds the decode's relative bound; first: cos({x:e}) = {g:e} against f64 \
             {:e}, relative error {d:e} ({:.1} f32 eps) against {:e} (ε_phys/4)",
            f64::from(x).cos(),
            d / f64::from(f32::EPSILON),
            bound()
        );
    }
}

#[test]
fn gpu_cos_near_half_pi() {
    let h = GpuHarness::new().expect("a GPU device");
    eprintln!("{}", h.adapter_info());
    let wgsl = wgsl();
    let xs = arguments();
    assert!(
        f64::from(TOP) < std::f64::consts::FRAC_PI_2 && f64::from(TOP).cos() < 1e-7,
        "the sweep's top {TOP:e} is not the f32 just below π/2"
    );
    assert_eq!(xs.len(), (TOP.to_bits() - BOTTOM.to_bits() + 1) as usize);
    for setting in [FastMath::default(), FastMath::On] {
        let got = kernel_gpu(&h, &wgsl, &xs, setting);
        let n = check_identity(&format!("the kernel's sin_cos, {setting:?}"), &xs, &got);
        eprintln!("the kernel's sin_cos, {setting:?}: {n} arguments, the CPU's bits");
        let cos: Vec<f32> = got.iter().map(|g| g.1).collect();
        let absolute = absolute_worst(&xs, &cos);
        eprintln!("the kernel's cos, {setting:?}: worst absolute error {absolute:e} (WGSL's bound {WGSL_ABSOLUTE:e})");
        check(&format!("the kernel's cos, {setting:?}"), &xs, &cos);
        let wide = circle_arguments();
        let got = kernel_gpu(&h, &wgsl, &wide, setting);
        let n = check_identity(
            &format!("the kernel's sin_cos on [−2π, 2π], {setting:?}"),
            &wide,
            &got,
        );
        eprintln!("the kernel's sin_cos on [−2π, 2π], {setting:?}: {n} arguments, the CPU's bits");
    }
}

/// Every 997th f32 in `[0, 2π̂]` and its negative, with `±0`: the decode's `β` after `on_circle`, both signs, each octant
/// and its swap past `π/2`, the tiny path below `2⁻¹²` and the reduction's `r = 0` window, so the bit identity the CPU's
/// exhaustive sweep rests on is a test here, not an argument.
fn circle_arguments() -> Vec<f32> {
    let top = (2.0 * std::f32::consts::PI).to_bits();
    (0..=top)
        .step_by(997)
        .chain([top])
        .map(f32::from_bits)
        .flat_map(|x| [x, -x])
        .collect()
}

negative_control!(
    gpu_cos_near_half_pi,
    "the kernel's entry with its cos word one ulp up is not the CPU's",
    expected = "are not the CPU's",
    {
        let h = GpuHarness::new().expect("a GPU device");
        let xs: Vec<f32> = arguments().into_iter().step_by(4099).collect();
        let got: Vec<(f32, f32)> = kernel_gpu(&h, &wgsl(), &xs, FastMath::default())
            .into_iter()
            .map(|(s, c)| (s, f32::from_bits(c.to_bits() + 1)))
            .collect();
        check_identity("one ulp up", &xs, &got);
    }
);

#[cfg(feature = "controls")]
mod absolute_eps {
    use super::*;

    validation::negative_control!(
        gpu_cos_near_half_pi,
        "a stand-in cos one f32 eps from f64 absolutely, inside WGSL's 2⁻¹¹, fails where cos x is small",
        expected = "exceeds the decode's relative bound",
        {
            let xs = arguments();
            let eps = f64::from(f32::EPSILON);
            let stand_in: Vec<f32> = xs
                .iter()
                .map(|&x| (f64::from(x).cos() + eps) as f32)
                .collect();
            assert!(absolute_worst(&xs, &stand_in) <= WGSL_ABSOLUTE);
            check("stand-in, f64 + 2⁻²³", &xs, &stand_in)
        }
    );
}

#[cfg(feature = "controls")]
mod half_angle {
    use super::*;

    validation::negative_control!(
        gpu_cos_near_half_pi,
        "the GPU's 1 − 2 sin²(x/2), inside WGSL's 2⁻¹¹ absolutely, fails where cos x is small",
        expected = "exceeds the decode's relative bound",
        {
            let h = GpuHarness::new().expect("a GPU device");
            let xs = arguments();
            let got = gpu(&h, HALF_ANGLE_WGSL, "half_angle", &xs, FastMath::default());
            let top = got[got.len() - 1];
            eprintln!(
                "stand-in at x = {TOP:e}: {top:e} ({:#010x}) against f64 {:e}",
                top.to_bits(),
                f64::from(TOP).cos()
            );
            assert!(absolute_worst(&xs, &got) <= WGSL_ABSOLUTE);
            check("stand-in, 1 − 2 sin²(x/2)", &xs, &got)
        }
    );
}
