//! The kernel's own f32 `atan2` on the GPU (`kernel::arctan`, `kernel::stage::encode::arctangent`,
//! `engine::decode_dispatch::arctan2_gpu`; RQ-628, decided by the orchestrator per R-369, option (a)): the kernel's f32
//! build, `target/spirv/kernel.wgsl`, dispatched natively under the default fast-math setting, Metal's fast-math on
//! (R-297), as the backend compiles it. CI runs this file in the `gpu-kernel` job, after `cargo xtask build-kernel`, on
//! lavapipe and Metal.
//!
//! The function is integer arithmetic alone (`kernel::arctan`'s note), so the GPU's result is the CPU's, bit for bit, and
//! the CPU's sweeps (`crates/kernel/tests/arctan.rs`) hold for it. Each test checks that identity, and prints the GPU's
//! accuracy against f64 `atan2` (`validation::arctan`).
//! - `arctan_gpu_is_cpu`: the special values' pairs, every 1021st `t ∈ [0, 1]` of the reduced sweep, 2¹⁸ plane pairs
//!   and the round trip's `(sin α, cos α)` for every 1021st `α ∈ [0, π/2]`, the decode's f32 `sin` and `cos` the CPU's.
//! - `arctan_gpu_exhaustive`, ignored: every f32 `t ∈ [0, 1]` and every round trip, in dispatches of 65535 × 64
//!   pairs, minutes on Metal (applied per R-369: too slow for CI; run locally with `cargo test -p engine --test
//!   arctan_gpu -- --ignored`).

use std::path::{Path, PathBuf};

use engine::contract::fast_math::FastMath;
use engine::decode_dispatch::arctan2_gpu;
use kernel::arctan::atan2_f32;
use validation::arctan::{alphas, plane, reduced, Accuracy, RoundTrip};
use validation::gpu::GpuHarness;
use validation::negative_control;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The WGSL `cargo xtask build-kernel` wrote.
fn wgsl() -> String {
    std::fs::read_to_string(root().join("target/spirv/kernel.wgsl"))
        .expect("target/spirv/kernel.wgsl: run `cargo xtask build-kernel` first")
}

/// The pairs one dispatch takes: a dispatch's most workgroups, 65535, of 64.
const BATCH: usize = 65535 * 64;

/// The special values' pairs: every pair of signed zeros, subnormals, normals, the largest finite values, infinities
/// and NaN.
fn specials() -> Vec<[f32; 2]> {
    let one = [
        0.0,
        f32::from_bits(1),
        f32::MIN_POSITIVE,
        1.0e-30,
        0.5,
        1.0,
        3.0,
        1.0e30,
        f32::MAX,
        f32::INFINITY,
    ];
    let mut all: Vec<f32> = one.iter().flat_map(|&v| [v, -v]).collect();
    all.push(f32::NAN);
    all.iter()
        .flat_map(|&y| all.iter().map(move |&x| [y, x]))
        .collect()
}

/// Checks the GPU's `atan2` on `yx`, dispatched in batches of [`BATCH`] pairs, against `cpu`'s, bit for bit (NaN for
/// NaN), and prints its accuracy; `each` sees every pair's tag, the pair and the GPU's result. No sweep is held in
/// memory whole.
fn check_same(
    h: &GpuHarness,
    wgsl: &str,
    sweep: &str,
    yx: impl Iterator<Item = (f32, [f32; 2])>,
    cpu: fn(f32, f32) -> f32,
    mut each: impl FnMut(f32, [f32; 2], f32),
) {
    let mut acc = Accuracy::default();
    let mut yx = yx.peekable();
    while yx.peek().is_some() {
        let (tags, batch): (Vec<f32>, Vec<[f32; 2]>) = yx.by_ref().take(BATCH).unzip();
        let got = arctan2_gpu(h.device(), h.queue(), wgsl, &batch, FastMath::default())
            .expect("the atan2 dispatch");
        assert_eq!(got.len(), batch.len(), "{sweep}: one result per pair");
        for ((&tag, &[y, x]), &g) in tags.iter().zip(&batch).zip(&got) {
            let want = cpu(y, x);
            assert!(
                g.to_bits() == want.to_bits() || (g.is_nan() && want.is_nan()),
                "{sweep}: the GPU's atan2({y:e}, {x:e}) = {g:e} ({:#010x}) differs from the CPU's {want:e} ({:#010x})",
                g.to_bits(),
                want.to_bits()
            );
            acc.add([y, x], g);
            each(tag, [y, x], g);
        }
    }
    let info = h.adapter_info();
    eprintln!(
        "{sweep}: the GPU's on {} ({:?}): {acc:?}",
        info.name, info.backend
    );
}

/// Checks the GPU against `cpu` on the special values, the reduced sweep at `stride`, `pairs` plane pairs and the round
/// trip at `stride`, the decode's f32 `sin` and `cos` the CPU's; prints the round trip's measures.
fn check_gpu(stride: u32, pairs: usize, cpu: fn(f32, f32) -> f32) {
    let h = GpuHarness::new().expect("a GPU device");
    let wgsl = wgsl();
    let none = |_, _, _| {};
    let untagged = |yx: [f32; 2]| (0.0, yx);
    check_same(
        &h,
        &wgsl,
        "special values",
        specials().into_iter().map(untagged),
        cpu,
        none,
    );
    check_same(
        &h,
        &wgsl,
        "reduced",
        reduced(stride).map(|t| (0.0, [t, 1.0])),
        cpu,
        none,
    );
    check_same(
        &h,
        &wgsl,
        "plane",
        plane(pairs, 0x0628_0002).into_iter().map(untagged),
        cpu,
        none,
    );
    let mut trip = RoundTrip::default();
    let yx = alphas(stride).map(|a| (a, [a.sin(), a.cos()]));
    check_same(&h, &wgsl, "round trip", yx, cpu, |a, [s, c], b| {
        trip.add(a, [s, c], b, [b.sin(), b.cos()]);
    });
    eprintln!(
        "round trip: the GPU's {trip:?}, share {:.6}%",
        100.0 * trip.share()
    );
}

#[test]
fn arctan_gpu_is_cpu() {
    check_gpu(1021, 1 << 18, atan2_f32);
}

negative_control!(
    arctan_gpu_is_cpu,
    "the GPU's results held to the CPU's libm atan2f, not the kernel's own",
    expected = "differs from the CPU's",
    check_gpu(1021, 0, f32::atan2)
);

#[test]
#[ignore = "every f32 t in [0, 1] and α in [0, π/2]: minutes on Metal (the module's note)"]
fn arctan_gpu_exhaustive() {
    check_gpu(1, 0, atan2_f32);
}

negative_control!(
    arctan_gpu_exhaustive,
    "the GPU's results held to the CPU's libm atan2f, on every 4096th t and α",
    expected = "differs from the CPU's",
    check_gpu(4096, 0, f32::atan2)
);
