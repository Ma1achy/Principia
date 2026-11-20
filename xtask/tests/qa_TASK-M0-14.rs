//! QA tests for TASK-M0-14's `cargo xtask build-kernel`, written from canonical_spec §1 items 2 and 3 (the kernel is
//! compiled "to **f32 / SPIR-V** for the GPU via rust-gpu", "SPIR-V → WGSL", and "the GPU is f32-locked") and the
//! task's acceptance line (rust-gpu compiles `crates/kernel` to SPIR-V and naga translates it to WGSL). Each test
//! registers a negative control (R-176).
//!
//! They read `target/spirv/kernel.spv` and `kernel.wgsl`, so `cargo xtask build-kernel` runs first, as CI runs it.

use std::fs;
use std::path::{Path, PathBuf};

use validation::negative_control;
use xtask::build_kernel::{to_wgsl, SPV, WGSL};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn spv_words() -> Vec<u32> {
    let bytes = fs::read(root().join(SPV))
        .expect("target/spirv/kernel.spv: run `cargo xtask build-kernel` first");
    assert_eq!(bytes.len() % 4, 0, "the SPIR-V module is not whole words");
    bytes
        .chunks_exact(4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect()
}

/// SPIR-V (Khronos spec §2.3, §3): the magic number; a compute (`GLCompute`, 5) entry point; no `Float64` capability
/// (10) and no 64-bit `OpTypeFloat` (22): an f32 module, as the f32-locked GPU needs.
fn check_f32_compute_module(words: &[u32]) {
    assert!(
        words.len() > 5 && words[0] == 0x0723_0203,
        "not a SPIR-V module (magic {:#010x})",
        words.first().copied().unwrap_or(0)
    );
    let (mut compute, mut at) = (false, 5);
    while at < words.len() {
        let count = (words[at] >> 16) as usize;
        let op = words[at] & 0xffff;
        assert!(
            count > 0 && at + count <= words.len(),
            "malformed instruction at word {at}"
        );
        let args = &words[at + 1..at + count];
        match op {
            17 => assert_ne!(
                args[0], 10,
                "the module declares the Float64 capability: not f32-locked"
            ),
            22 => assert_ne!(
                args[1], 64,
                "the module declares a 64-bit float type: not f32-locked"
            ),
            15 => compute |= args[0] == 5,
            _ => {}
        }
        at += count;
    }
    assert!(compute, "the module has no GLCompute entry point");
}

/// canonical_spec §1 items 2–3: the GPU build is an f32 SPIR-V compute module, and its WGSL names no f64.
#[test]
fn qa_build_kernel_spirv_is_an_f32_compute_module() {
    check_f32_compute_module(&spv_words());
    let wgsl = fs::read_to_string(root().join(WGSL)).expect("target/spirv/kernel.wgsl");
    assert!(!wgsl.contains("f64"), "the kernel's WGSL names f64");
}

negative_control!(
    qa_build_kernel_spirv_is_an_f32_compute_module,
    "the kernel's module with `OpCapability Float64` appended",
    expected = "declares the Float64 capability",
    {
        let mut words = spv_words();
        words.extend([(2 << 16) | 17, 10]);
        check_f32_compute_module(&words)
    }
);

/// The WGSL on disk is `text`, naga's translation of the SPIR-V on disk, byte for byte.
fn check_is_translation(translated: &str, on_disk: &str) {
    assert!(
        translated == on_disk,
        "target/spirv/kernel.wgsl is not naga's translation of target/spirv/kernel.spv"
    );
}

/// The acceptance line: the WGSL the GPU dispatches is naga's translation of the SPIR-V rust-gpu built, the pair
/// written by one run, not a WGSL from elsewhere.
#[test]
fn qa_build_kernel_wgsl_is_the_spirv_translated() {
    let spv = fs::read(root().join(SPV)).expect("target/spirv/kernel.spv");
    let on_disk = fs::read_to_string(root().join(WGSL)).expect("target/spirv/kernel.wgsl");
    let translated = to_wgsl(&spv).unwrap_or_else(|e| panic!("{e}"));
    check_is_translation(&translated, &on_disk);
}

negative_control!(
    qa_build_kernel_wgsl_is_the_spirv_translated,
    "the translation with its workgroup size changed, which is not the WGSL on disk",
    expected = "is not naga's translation",
    {
        let spv = fs::read(root().join(SPV)).expect("target/spirv/kernel.spv");
        let on_disk = fs::read_to_string(root().join(WGSL)).expect("target/spirv/kernel.wgsl");
        let translated = to_wgsl(&spv)
            .unwrap()
            .replace("workgroup_size(64", "workgroup_size(32");
        check_is_translation(&translated, &on_disk)
    }
);
