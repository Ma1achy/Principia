//! naga validation of the kernel's translation refuses a non-finite float literal (gpu_determinism_note § "The
//! discipline", rule 4): `cargo xtask build-kernel` translates the kernel's SPIR-V through `to_wgsl`, which validates
//! with every flag, so a kernel holding an `f32::INFINITY` or NaN constant fails its build.
//!
//! The plan is unclear here: it puts this test in `crates/kernel/tests/naga_validate.rs`; chose xtask because the test
//! must run the build's own translation, `xtask::build_kernel::to_wgsl`, which the kernel's tests cannot reach. It
//! needs no built kernel, so it runs on every PR; `build_kernel_translates_the_kernel` (`tests/build_kernel.rs`, the
//! gpu-kernel job) validates the real module through the same function. The all-unusable fold is
//! `crates/kernel/tests/regularisation.rs`'s `dt_max` tests.

use validation::negative_control;
use xtask::build_kernel::to_wgsl;

/// A minimal compute module, hand-assembled SPIR-V 1.3: one entry `main` storing the f32 constant whose bits are `bits`
/// into a private variable, so the constant survives the translation's compaction.
fn storing_constant(bits: u32) -> Vec<u8> {
    // The header: magic, version 1.3, generator, id bound, schema.
    let mut words = vec![0x0723_0203, 0x0001_0300, 0, 9, 0];
    let mut op = |code: u32, operands: &[u32]| {
        words.push(((operands.len() as u32 + 1) << 16) | code);
        words.extend_from_slice(operands);
    };
    op(17, &[1]); // OpCapability Shader
    op(14, &[0, 1]); // OpMemoryModel Logical GLSL450
    op(15, &[5, 7, u32::from_le_bytes(*b"main"), 0]); // OpEntryPoint GLCompute %7 "main"
    op(16, &[7, 17, 1, 1, 1]); // OpExecutionMode %7 LocalSize 1 1 1
    op(19, &[1]); // %1 = OpTypeVoid
    op(33, &[2, 1]); // %2 = OpTypeFunction %1
    op(22, &[3, 32]); // %3 = OpTypeFloat 32
    op(43, &[3, 4, bits]); // %4 = OpConstant %3 bits
    op(32, &[5, 6, 3]); // %5 = OpTypePointer Private %3
    op(59, &[5, 6, 6]); // %6 = OpVariable %5 Private
    op(54, &[1, 7, 0, 2]); // %7 = OpFunction %1 None %2
    op(248, &[8]); // %8 = OpLabel
    op(62, &[6, 4]); // OpStore %6 %4
    op(253, &[]); // OpReturn
    op(56, &[]); // OpFunctionEnd
    words.iter().flat_map(|w| w.to_le_bytes()).collect()
}

/// The translation refuses a module holding an f32 constant of the bits `bits`, a non-finite literal, naming `what`:
/// naga's validator, run by every build-kernel translation, rejects it (gpu_determinism_note § "The discipline", rule 4).
fn check_literal_refused(bits: u32, what: &str) {
    match to_wgsl(&storing_constant(bits)) {
        Ok(_) => panic!("a module holding the f32 literal {bits:#010x} translated"),
        Err(e) => assert!(
            e.contains(what),
            "{bits:#010x} refused, but not as {what}: {e}"
        ),
    }
}

#[test]
fn naga_validate_refuses_non_finite_literals() {
    // The control arm: the same module with a finite constant translates, so a refusal below is the literal's.
    if let Err(e) = to_wgsl(&storing_constant(1.0f32.to_bits())) {
        panic!("the finite module did not translate: {e}");
    }
    check_literal_refused(f32::INFINITY.to_bits(), "Literal(Infinity)");
    check_literal_refused(f32::NEG_INFINITY.to_bits(), "Literal(Infinity)");
    check_literal_refused(f32::NAN.to_bits(), "Literal(NaN)");
}

negative_control!(
    naga_validate_refuses_non_finite_literals,
    "a finite literal, 1.0, taken for a non-finite one",
    expected = "translated",
    check_literal_refused(1.0f32.to_bits(), "Literal(Infinity)")
);
