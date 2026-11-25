//! The backend's builtin WGSL `cos` near `π/2` and `sin` near 0, against f64, relative to the value: measured and
//! printed, never held (the decode calls the kernel's own, `kernel::trig`, which `cos_half_pi_gpu.rs` holds). No kernel
//! build is needed, so CI's `ci` profile runs this on lavapipe; the Windows card runs it under `win-gpu-kernel`.
//!
//! - `gpu_builtin_cos_near_half_pi_measured`: every f32 from `π/4` to `LIT_6`, the f32 below `π/2`, as
//!   `cos_half_pi_gpu.rs` sweeps them, under each fast-math setting.
//! - `gpu_builtin_sin_near_zero_measured`: every 16th f32 from `2⁻²⁴` to `π/4`, the reduced arguments a decode built on
//!   `cos α = sin(π/2 − α)` would hand the builtin. It is what would have to stay relative near 0 for that form.
//!
//! Each prints the worst relative error per band of the value, `[2⁻⁽ᵏ⁺¹⁾, 2⁻ᵏ)`, in f32 eps, and the count past
//! `ε_phys/4` (16 eps), the decode's bound. What each holds is only that the report read every argument: one result
//! per argument, each counted in a band; its control drops one result.

use engine::contract::fast_math::FastMath;
use kernel::generated::links::Literals;
use ledger::constants::EPS_PHYS;
use validation::gpu::GpuHarness;

/// The f32 below `π/2`.
const TOP: f32 = <f32 as Literals>::LIT_6;

/// The arguments one dispatch takes, under a dispatch's 65535 workgroups of 64.
const BATCH: usize = 1 << 20;

/// The bands of the value the report reads.
const BANDS: usize = 26;

const WGSL: &str = r"
@group(0) @binding(0) var<storage, read> x: array<u32>;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;
@compute @workgroup_size(64)
fn cosine(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x < arrayLength(&x)) { output[id.x] = bitcast<u32>(cos(bitcast<f32>(x[id.x]))); }
}
@compute @workgroup_size(64)
fn sine(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x < arrayLength(&x)) { output[id.x] = bitcast<u32>(sin(bitcast<f32>(x[id.x]))); }
}
";

/// The decode's relative bound, `ε_phys/4`.
fn bound() -> f64 {
    EPS_PHYS.number() * f64::from(f32::EPSILON) / 4.0
}

/// `entry` of each argument, under `setting`, in batches of [`BATCH`].
fn gpu(h: &GpuHarness, entry: &str, xs: &[f32], setting: FastMath) -> Vec<f32> {
    let mut out = Vec::with_capacity(xs.len());
    for chunk in xs.chunks(BATCH) {
        let words: Vec<u32> = chunk.iter().map(|x| x.to_bits()).collect();
        let got = h.run_wgsl_with(WGSL, entry, &[&words], setting);
        assert_eq!(got.len(), chunk.len(), "{entry}: one result per argument");
        out.extend(got.into_iter().map(f32::from_bits));
    }
    out
}

/// Prints the worst relative error of `got` against `exact` per band of the value, in f32 eps, and the count past
/// [`bound`].
fn report(label: &str, xs: &[f32], got: &[f32], exact: fn(f64) -> f64) {
    assert_eq!(
        got.len(),
        xs.len(),
        "{label}: the report did not read one result per argument"
    );
    let eps = f64::from(f32::EPSILON);
    let mut bands = [(0usize, 0.0f64, 0.0f32); BANDS];
    let (mut over, mut absolute) = (0, 0.0f64);
    for (&x, &g) in xs.iter().zip(got) {
        let want = exact(f64::from(x));
        let d = (f64::from(g) - want).abs() / want.abs();
        absolute = absolute.max((f64::from(g) - want).abs());
        let k = (-want.abs().log2()).floor().max(0.0) as usize;
        let band = &mut bands[k.min(BANDS - 1)];
        band.0 += 1;
        if d.is_nan() || d > band.1 {
            (band.1, band.2) = (d, x);
        }
        if d.is_nan() || d > bound() {
            over += 1;
        }
    }
    eprintln!("{label}: worst relative error per band of the value, in f32 eps (the decode's bound {:.1} eps)", bound() / eps);
    for (k, &(n, worst, at)) in bands.iter().enumerate().filter(|b| b.1 .0 > 0) {
        eprintln!(
            "  value in [2^-{}, 2^-{k}): {n:>9} args, worst {:>14.4} eps at x = {at:e} ({:#010x})",
            k + 1,
            worst / eps,
            at.to_bits()
        );
    }
    eprintln!(
        "{label}: {over} of {} past the bound; worst absolute error {absolute:e}",
        xs.len()
    );
    let counted: usize = bands.iter().map(|b| b.0).sum();
    assert_eq!(
        counted,
        xs.len(),
        "{label}: the report did not read one result per argument"
    );
}

validation::negative_control!(
    gpu_builtin_cos_near_half_pi_measured,
    "a result dropped",
    expected = "the report did not read one result per argument",
    report("dropped", &[0.5, 1.0], &[0.5f32.cos()], f64::cos)
);

validation::negative_control!(
    gpu_builtin_sin_near_zero_measured,
    "a result dropped",
    expected = "the report did not read one result per argument",
    report("dropped", &[0.5, 1.0], &[0.5f32.sin()], f64::sin)
);

#[test]
fn gpu_builtin_cos_near_half_pi_measured() {
    let h = GpuHarness::new().expect("a GPU device");
    eprintln!("{}", h.adapter_info());
    let xs: Vec<f32> = (std::f32::consts::FRAC_PI_4.to_bits()..=TOP.to_bits())
        .map(f32::from_bits)
        .collect();
    for setting in [FastMath::default(), FastMath::On] {
        let got = gpu(&h, "cosine", &xs, setting);
        report(
            &format!("builtin cos, {setting:?} (measured, not held)"),
            &xs,
            &got,
            f64::cos,
        );
    }
}

#[test]
fn gpu_builtin_sin_near_zero_measured() {
    let h = GpuHarness::new().expect("a GPU device");
    eprintln!("{}", h.adapter_info());
    let xs: Vec<f32> = (2f32.powi(-24).to_bits()..=std::f32::consts::FRAC_PI_4.to_bits())
        .step_by(16)
        .map(f32::from_bits)
        .collect();
    for setting in [FastMath::default(), FastMath::On] {
        let got = gpu(&h, "sine", &xs, setting);
        report(
            &format!("builtin sin, {setting:?} (measured, not held)"),
            &xs,
            &got,
            f64::sin,
        );
    }
}
