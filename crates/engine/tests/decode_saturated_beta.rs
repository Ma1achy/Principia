//! The decode's `β` at the saturated controls, natively at f32: the samples of `qa_TASK-M2-06_gpu`'s
//! `qa_m206_gpu_saturated_controls` (`saturated/mod.rs`), both compiled variants, on the CPU.
//!
//! **What failed.** On lavapipe and on Metal, sample 410 of the defaults (every latent component `+10³⁰`, the centre)
//! decoded with `I − 1 = 8388607.9999999525` f32 eps: `I ≈ 5.7·10⁻¹⁵`, `‖ρ̃‖²` alone, so the GPU's `λ̃ = sin α·(cos β,
//! sin β)` was `(0, 0)`. The periodic `β = z_β + π/2` reached the backend's `cos` and `sin` unreduced, at about `10³⁰`,
//! where WGSL bounds neither. The CPU's f32 `cos` and `sin` reduce exactly, so no CPU output was wrong: what the CPU can
//! see is the argument the decode hands them, which must lie in `[−π̂, π̂]`, the one period WGSL bounds, and be `β`
//! modulo `2π` there.
//!
//! - `saturated_beta_on_circle_f32`: at every sample, the decode's `β` read on the circle (`Real::on_circle`) lies in
//!   `[−π̂, π̂]` and is within half an f32 step (plus `2⁻³⁸` relative, `kernel::circle`'s caveat at a midpoint) of
//!   `β mod 2π` from the pinned f64 libm at the widened `β`; and `I − 1` is within 8 f32 eps, the GPU test's
//!   `INERTIA_EPS`. Control: the decode before the fix, `β` handed on unreduced.
//! - `sample_410_beta_on_circle_f32`: the same at sample 410 alone, the GPU's failure. Control: sample 410's `β = 10³⁰`
//!   handed on unreduced, as before the fix.
//! - `saturated_lambda_reads_beta_on_circle_f32`: the decode's `λ̃` is `sin α·(cos c, sin c)` at `c` the reduced `β`,
//!   bit for bit, so the decoder reads the reduction, not `β`; and the decode the test reads is the stage's, `(m, r, p)`
//!   bit for bit. Control: `λ̃` expected from the unreduced `β`, as before the fix.
//!
//! **Domain** (pitfalls §10): the decode alone, its inputs held fixed; no trajectory.

use kernel::chart::{Chart, Latent};
use kernel::circle::PI_HAT_BITS;
use kernel::stage::decode::{sample, DecodeVariant, LatentDefaults, LatentStickBreaking};
use kernel::Real;
use validation::negative_control;

mod saturated;
use saturated::saturated;

const EPS: f64 = f32::EPSILON as f64;

/// The GPU test's bound on `I − 1`, in f32 eps (`qa_TASK-M2-06_gpu`'s `INERTIA_EPS`).
const INERTIA_EPS: f64 = 8.0;

/// Sample 410: the defaults' first latent at `10³⁰` (82 samples per magnitude, the sixth magnitude), at the centre.
const SAMPLE_410: usize = 410;

/// `β mod 2π` in `(−π, π]` at f64, from the pinned libm's exact reductions at the widened `β`.
fn reference(beta: f32) -> f64 {
    let b = f64::from(beta);
    Real::arctangent2(Real::sine(b), Real::cosine(b))
}

/// Every sample of [`saturated`] through variant `V`: its index, its label, the chart and `(u, v)`.
fn samples<V: DecodeVariant<Chart = Latent<f32>>>() -> Vec<(usize, String, Latent<f32>, [f32; 2])> {
    let mut out = Vec::new();
    for (chart, uv) in saturated() {
        for p in uv {
            let i = out.len();
            out.push((i, format!("{} sample {i}", V::ENTRY), chart, p));
        }
    }
    out
}

/// `reduce`, standing for the angle the decode hands `cos` and `sin`, at every sample through `V`: in `[−π̂, π̂]`,
/// within half an f32 step (plus `2⁻³⁸` relative and `10⁻¹⁵`) of `β mod 2π`, and `I − 1` within [`INERTIA_EPS`].
/// `only`, if given, restricts the check to that one sample.
fn check_on_circle<V: DecodeVariant<Chart = Latent<f32>>>(
    reduce: fn(f32) -> f32,
    only: Option<usize>,
) {
    for (i, at, chart, [u, v]) in samples::<V>() {
        if only.is_some_and(|k| k != i) {
            continue;
        }
        let d = Chart::decode::<V::Links, f32>(&chart, u, v);
        let beta = d.config.beta;
        let c = reduce(beta);
        assert!(
            c.to_bits() & 0x7FFF_FFFF <= PI_HAT_BITS,
            "the angle the decode hands cos and sin, {c:e} for β = {beta:e}, is outside [−π̂, π̂] at {at}"
        );
        let want = reference(beta);
        let two_pi = 2.0 * core::f64::consts::PI;
        let diff = f64::from(c) - want;
        let diff = (diff - two_pi * (diff / two_pi).round()).abs();
        let step = f64::from(f32::from_bits(c.abs().to_bits().max(1) + 1)) - f64::from(c.abs());
        let bound = 0.5 * step + want.abs() * 2f64.powi(-38) + 1e-15;
        assert!(
            diff <= bound,
            "the angle {c:e} for β = {beta:e} is {diff:e} from β mod 2π = {want:e}, past {bound:e}, at {at}"
        );
        let (m, r) = (d.ic.m, d.ic.r);
        let inertia: f64 = (0..3)
            .map(|k| f64::from(m[k]) * (f64::from(r[k][0]).powi(2) + f64::from(r[k][1]).powi(2)))
            .sum();
        let e = (inertia - 1.0).abs() / EPS;
        assert!(e <= INERTIA_EPS, "I − 1 = {e} eps at {at}");
        if i == SAMPLE_410 && V::ENTRY == LatentDefaults::ENTRY {
            assert_eq!(
                chart.z0, [1e30; 8],
                "sample 410 is not the all-+10³⁰ latent"
            );
            println!(
                "{at}: z = +1e30 at (u, v) = ({u}, {v}); β = {beta:e}, on the circle {c:e} (β mod 2π = {want:e}); \
                 λ̃ = {:?}; ρ̃ = {:?}; I − 1 = {e:.3} eps; tag {:?}",
                d.config.lambda_t, d.config.rho_t, d.degenerate
            );
        }
    }
}

#[test]
fn saturated_beta_on_circle_f32() {
    check_on_circle::<LatentDefaults>(Real::on_circle, None);
    check_on_circle::<LatentStickBreaking>(Real::on_circle, None);
}

negative_control!(
    saturated_beta_on_circle_f32,
    "the decode before the fix, which handed β to cos and sin unreduced",
    expected = "is outside [−π̂, π̂]",
    check_on_circle::<LatentDefaults>(|b| b, None)
);

#[test]
fn sample_410_beta_on_circle_f32() {
    check_on_circle::<LatentDefaults>(Real::on_circle, Some(SAMPLE_410));
}

negative_control!(
    sample_410_beta_on_circle_f32,
    "sample 410 as the decode handed it before the fix: β = 10³⁰ unreduced",
    expected = "is outside [−π̂, π̂] at stage_decode_latent_defaults sample 410",
    check_on_circle::<LatentDefaults>(|b| b, Some(SAMPLE_410))
);

/// At every sample through `V`: the stage's `(m, r, p)` is the decode's that the test reads, bit for bit; and the
/// decode's `λ̃` is `sin α·(cos c, sin c)` at `c = reduce(β)`, bit for bit. Prints how many samples' `λ̃` would differ
/// had the decode read `β` unreduced.
fn check_lambda<V: DecodeVariant<Chart = Latent<f32>>>(reduce: fn(f32) -> f32) {
    let mut moved = 0;
    let all = samples::<V>();
    for (_, at, chart, [u, v]) in &all {
        let d = Chart::decode::<V::Links, f32>(chart, *u, *v);
        let n = sample::<_, V::Links, f32>(chart, *u, *v);
        assert_eq!(
            format!("{:?}", n.ic),
            format!("{:?}", d.ic),
            "the stage's (m, r, p) is not the decode's at {at}"
        );
        let cfg = d.config;
        let lambda = |c: f32| {
            let s = Real::sine(cfg.alpha);
            [s * Real::cosine(c), s * Real::sine(c)]
        };
        let want = lambda(reduce(cfg.beta));
        assert_eq!(
            cfg.lambda_t.map(f32::to_bits),
            want.map(f32::to_bits),
            "the decode's λ̃ {:?} is not read from β on the circle ({want:?}) at {at}",
            cfg.lambda_t
        );
        if lambda(cfg.beta).map(f32::to_bits) != want.map(f32::to_bits) {
            moved += 1;
        }
        if at.ends_with(" sample 410") && V::ENTRY == LatentDefaults::ENTRY {
            println!(
                "{at}: λ̃ from β unreduced {} the reduced λ̃ in its bits (the CPU's own f32 cos and sin reduce exactly)",
                if lambda(cfg.beta) == want { "matches" } else { "differs from" }
            );
        }
    }
    println!(
        "{}: {} samples; λ̃ from β unreduced would differ in its bits at {moved}",
        V::ENTRY,
        all.len()
    );
}

#[test]
fn saturated_lambda_reads_beta_on_circle_f32() {
    check_lambda::<LatentDefaults>(Real::on_circle);
    check_lambda::<LatentStickBreaking>(Real::on_circle);
}

negative_control!(
    saturated_lambda_reads_beta_on_circle_f32,
    "λ̃ expected from β unreduced, as the decode read it before the fix",
    expected = "is not read from β on the circle",
    check_lambda::<LatentDefaults>(|b| b)
);
