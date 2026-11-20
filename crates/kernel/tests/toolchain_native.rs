//! The trivial kernel's native build (canonical_spec §1 item 2): `kernel::toolchain::pack_unpack`, called once per word
//! as the GPU dispatches it, writes the generated pack/unpack of each word (payload §2, §6), and an invocation past the
//! end of the buffer touches nothing. `validation`'s `toolchain_trivial_kernel` compares these words with the GPU's.

use kernel::toolchain::{pack_unpack, UVec3};
use validation::gpu::identity_fixture;
use validation::negative_control;

/// The kernel run natively over `input`, one call of the entry point per word.
fn native(input: &[u32]) -> Vec<u32> {
    let mut output = vec![0; input.len()];
    for i in 0..input.len() as u32 {
        pack_unpack(UVec3::new(i, 0, 0), input, &mut output);
    }
    output
}

/// Each word of `output` is the pack/unpack of `input`'s: at an even index `packed_a`'s descriptor, bits 0–9, over
/// zero; at an odd one `times`, both halves kept.
fn check_pack_unpack(input: &[u32], output: &[u32]) {
    assert_eq!(input.len(), output.len());
    for (i, (&w, &out)) in input.iter().zip(output).enumerate() {
        let want = if i % 2 == 0 { w & 0x3ff } else { w };
        assert_eq!(out, want, "word {i} is not the pack/unpack of {w:#010x}");
    }
}

#[test]
fn toolchain_kernel_native_pack_unpack() {
    let input = identity_fixture();
    let output = native(&input);
    check_pack_unpack(&input, &output);
    // An invocation past the end of the buffer, as a GPU's last workgroup runs, reads and writes nothing.
    let mut after = output.clone();
    pack_unpack(UVec3::new(input.len() as u32, 0, 0), &input, &mut after);
    assert_eq!(
        after, output,
        "an invocation past the end wrote to the buffer"
    );
}

negative_control!(
    toolchain_kernel_native_pack_unpack,
    "the input words themselves, unpacked but never packed again",
    expected = "is not the pack/unpack of",
    {
        let input = identity_fixture();
        check_pack_unpack(&input, &input)
    }
);
