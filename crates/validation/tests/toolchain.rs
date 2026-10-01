//! The substrate toolchain (canonical_spec §1 item 2; parity_contract §6, Tier B): `kernel::toolchain`'s trivial kernel,
//! the generated pack/unpack of the packed words, run natively through rustc and dispatched on the GPU from the WGSL
//! that `cargo xtask build-kernel` writes (rust-gpu → SPIR-V → naga → WGSL), in one process, the packed words compared
//! bit for bit over 2¹⁶ inputs.
//!
//! The kernel is stateless, so this certifies the toolchain and the compiled pack/unpack given identical inputs, not
//! parity along a trajectory (pitfalls §10; M4).

use kernel::toolchain::{pack_unpack, UVec3};
use validation::gpu::{first_mismatch, identity_fixture, GpuHarness};
use validation::negative_control;

/// The WGSL entry point naga names for `kernel::toolchain::pack_unpack` (rust-gpu names it by its module path).
const ENTRY: &str = "toolchain_pack_unpack";

/// The WGSL `cargo xtask build-kernel` wrote to `target/spirv/kernel.wgsl`.
fn kernel_wgsl() -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/spirv/kernel.wgsl"
    );
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{path}: {e}; run `cargo xtask build-kernel` first"))
}

/// `wgsl` mis-built: the kernel's read of `detail`, bits 3–4 of `packed_a` (payload §2), shifted to bit 4. Panics
/// unless naga's WGSL holds exactly one such read, so a change in its output cannot leave the variant unshifted.
fn misbuilt(wgsl: &str) -> String {
    let read = ">> bitcast<u32>(3u)) & 3u)";
    assert_eq!(
        wgsl.matches(read).count(),
        1,
        "the WGSL has no single read of `detail` to shift"
    );
    wgsl.replacen(read, ">> bitcast<u32>(4u)) & 3u)", 1)
}

/// The kernel run natively over `input`, one call of the entry point per word, as the GPU dispatches it.
fn native(input: &[u32]) -> Vec<u32> {
    let mut output = vec![0; input.len()];
    for i in 0..input.len() as u32 {
        pack_unpack(UVec3::new(i, 0, 0), input, &mut output);
    }
    output
}

/// The kernel dispatched on the GPU from `wgsl` over `input`, through the harness.
fn gpu(wgsl: &str, input: &[u32]) -> Vec<u32> {
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    eprintln!("{}", h.adapter_info());
    h.run_wgsl(wgsl, ENTRY, &[input])
}

/// Tier B: the GPU's packed words are the native ones, bit for bit.
fn check_bit_identical(native: &[u32], gpu: &[u32]) {
    if let Some(i) = first_mismatch(native, gpu) {
        panic!(
            "GPU and native packed words differ at word {i}: native {:#010x}, GPU {:#010x}",
            native[i], gpu[i]
        );
    }
}

#[test]
fn toolchain_trivial_kernel() {
    let input = identity_fixture();
    assert_eq!(input.len(), 1 << 16);
    let native = native(&input);
    // The native words are the pack/unpack's: `packed_a`'s descriptor, bits 0–9, over zero; `times` unchanged.
    for (i, (&w, &out)) in input.iter().zip(&native).enumerate() {
        let want = if i % 2 == 0 { w & 0x3ff } else { w };
        assert_eq!(
            out, want,
            "native word {i} is not the pack/unpack of {w:#010x}"
        );
    }
    let wgsl = kernel_wgsl();
    check_bit_identical(&native, &gpu(&wgsl, &input));
    // The comparison can fire: the build with `detail`'s offset shifted differs from the native words.
    let bad = gpu(&misbuilt(&wgsl), &input);
    assert!(
        first_mismatch(&native, &bad).is_some(),
        "the mis-built GPU variant matched the native words"
    );
}

negative_control!(
    toolchain_trivial_kernel,
    "the GPU build with detail's offset shifted must differ from the native words",
    expected = "GPU and native packed words differ",
    {
        let input = identity_fixture();
        check_bit_identical(&native(&input), &gpu(&misbuilt(&kernel_wgsl()), &input))
    }
);
