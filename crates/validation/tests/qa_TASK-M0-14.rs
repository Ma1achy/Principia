//! QA tests for TASK-M0-14, written from the task's acceptance lines, REQ-PAY-017 and REQ-PAY-087, canonical_spec §1
//! item 2 (one kernel source, compiled to SPIR-V by rust-gpu and natively by rustc), parity_contract §6 (native
//! in-process harness) and Tier B ("all packed descriptor fields … and their bit offsets", the integer step indices,
//! bit-for-bit given the same inputs), and dd_simstate_payload §1 (the width function) and §2 (the bit layouts). The
//! expected words come from payload §2's bit table, not from the implementation. Each test registers a negative
//! control (R-176).
//!
//! The kernel is stateless (pitfalls §10): these tests certify the toolchain and the compiled pack/unpack given
//! identical inputs, not parity along a trajectory.
//!
//! The GPU tests read `target/spirv/kernel.wgsl`, which `cargo xtask build-kernel` writes (CI runs it before the tests).

use kernel::payload::PAYLOAD_LAYOUTS;
use kernel::toolchain::{pack_unpack, UVec3};
use ledger::gen::rust::{is_real, layout, precisions, Precision};
use validation::gpu::{first_mismatch, GpuHarness};
use validation::negative_control;

// ------------------------------------------------------------------------------------------------------------------
// The width function (REQ-PAY-017; dd_simstate_payload §1 "Layout at a `Real` of `w` bytes")
// ------------------------------------------------------------------------------------------------------------------

/// The ledger's `SimState` structs, which are the ones generic over the `Real`.
fn real_structs() -> Vec<ledger::schema::Struct> {
    ledger::payload::structs()
        .into_iter()
        .filter(is_real)
        .collect()
}

/// §1: a `SimState` struct storing `reals` `Real`s of `w` bytes, aligned to `a`, has `reals − 1` of them before the
/// packed words, 16 B of u32s, `closure_min` at its alignment, then 4 B of u16s: `(packed_a, closure_min, end, size,
/// align)`, the size rounded up to 8 or to `a` if greater.
fn s1(reals: u32, w: u32, a: u32) -> (u32, u32, u32, u32, u32) {
    let packed_a = (reals - 1) * w;
    let closure_min = (packed_a + 16).next_multiple_of(a);
    let end = closure_min + w + 4;
    let align = a.max(8);
    (
        packed_a,
        closure_min,
        end,
        end.next_multiple_of(align),
        align,
    )
}

/// The ledger's layout of each `SimState` struct at a precision of `w` bytes aligned to `a` is §1's.
fn check_width_function(w: u32, a: u32) {
    let p = Precision {
        name: "qa",
        size: w,
        align: a,
        instantiated: false,
    };
    for s in real_structs() {
        let reals = if s.name == "SimStateFTLE" { 31 } else { 19 };
        let l = layout(&s, &p);
        let at = |name: &str| {
            let k = s.members.iter().position(|m| m.name == name).unwrap();
            l.offsets[k]
        };
        assert_eq!(
            (at("packed_a"), at("closure_min"), l.end, l.size, l.align),
            s1(reals, w, a),
            "{}'s layout at a {w}-B Real aligned to {a} is not dd_simstate_payload §1's",
            s.name
        );
        // §1: "they end at `31w + 20` and `19w + 20`" whenever `closure_min` needs no padding before it.
        if (at("packed_a") + 16).is_multiple_of(a) {
            assert_eq!(
                l.end,
                reals * w + 20,
                "{} does not end at {reals}w + 20",
                s.name
            );
        }
    }
}

/// REQ-PAY-017: the generated table is the ledger's width function at each precision row, and that function is §1's at
/// widths no row uses yet, so a new row (philosophy §7.1: "a `DoubleF64` impl is another row") needs no fork.
#[test]
fn qa_generated_table_is_the_width_function() {
    let rows = precisions();
    assert_eq!(
        rows.len(),
        PAYLOAD_LAYOUTS.len(),
        "the generated table has a row the ledger lacks"
    );
    for (p, g) in rows.iter().zip(PAYLOAD_LAYOUTS.iter()) {
        let from_ledger: Vec<(&str, usize, usize)> = real_structs()
            .iter()
            .map(|s| {
                let l = layout(s, p);
                (s.name, l.size as usize, l.align as usize)
            })
            .collect();
        assert_eq!(
            (
                p.name,
                p.size as usize,
                p.align as usize,
                p.instantiated,
                from_ledger.as_slice()
            ),
            (
                g.real,
                g.real_size,
                g.real_align,
                g.instantiated,
                &g.structs[..]
            ),
            "the generated {} row is not the ledger's width function at it",
            g.real
        );
        check_width_function(p.size, p.align);
    }
    // Widths beyond the rows: a 4-, 8-, 16- or 32-byte Real at 4-, 8- or 16-byte alignment.
    for (w, a) in [(4, 4), (8, 4), (8, 8), (16, 8), (16, 16), (32, 8), (32, 16)] {
        check_width_function(w, a);
    }
}

negative_control!(
    qa_generated_table_is_the_width_function,
    "§1's function at 8 B given the ledger's layout at 4 B, as a width hardcoded to f32 would give",
    expected = "is not dd_simstate_payload §1's",
    {
        let p = Precision {
            name: "qa",
            size: 4,
            align: 4,
            instantiated: false,
        };
        let s = &real_structs()[0];
        let l = layout(s, &p);
        assert_eq!(
            (l.end, l.size),
            (s1(31, 8, 8).2, s1(31, 8, 8).3),
            "{}'s layout at an 8-B Real is not dd_simstate_payload §1's",
            s.name
        );
    }
);

// ------------------------------------------------------------------------------------------------------------------
// The trivial kernel, native and on the GPU (acceptance `toolchain_trivial_kernel`; parity_contract §6, Tier B)
// ------------------------------------------------------------------------------------------------------------------

/// The WGSL `cargo xtask build-kernel` wrote.
fn kernel_wgsl() -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/spirv/kernel.wgsl"
    );
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{path}: {e}; run `cargo xtask build-kernel` first"))
}

/// naga's name for `kernel::toolchain::pack_unpack`.
const ENTRY: &str = "toolchain_pack_unpack";

/// A fixture independent of the implementation's: 0, all ones, every single-bit word and every single-bit-cleared
/// word, each twice (so each lands at an even index, read as `packed_a`, and at an odd one, read as `times`), then
/// xorshift32 words, `len` in all.
fn qa_words(len: usize) -> Vec<u32> {
    let mut v = Vec::new();
    for w in [0u32, u32::MAX]
        .into_iter()
        .chain((0..32).map(|b| 1u32 << b))
        .chain((0..32).map(|b| !(1u32 << b)))
    {
        v.push(w);
        v.push(w);
    }
    let mut x = 0x2545_F491u32;
    while v.len() < len {
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        v.push(x);
    }
    v.truncate(len);
    v
}

/// payload §2's pack∘unpack of `w` at index `i`: an even index reads `packed_a`, whose descriptor is bits 0–9
/// (`state` 0–2, `detail` 3–4, `saturated` 5, `dmin_pair` 6–7, `last_symbol` 8–9), repacked over zero; an odd one reads
/// `times`, `t_end_step` bits 0–15 and `t_dmin_step` bits 16–31, repacked whole.
fn spec_word(i: usize, w: u32) -> u32 {
    if i.is_multiple_of(2) {
        w & 0x3ff
    } else {
        w
    }
}

/// The kernel run natively over `input`, one entry-point call per word.
fn native(input: &[u32]) -> Vec<u32> {
    let mut out = vec![0; input.len()];
    for i in 0..input.len() as u32 {
        pack_unpack(UVec3::new(i, 0, 0), input, &mut out);
    }
    out
}

/// `got` is payload §2's pack∘unpack of `input`, word for word; `who` names the build.
fn check_spec(who: &str, input: &[u32], got: &[u32]) {
    assert_eq!(input.len(), got.len(), "{who}: output length differs");
    for (i, (&w, &g)) in input.iter().zip(got).enumerate() {
        assert_eq!(
            g,
            spec_word(i, w),
            "{who} word {i} ({w:#010x}) is not payload §2's pack/unpack"
        );
    }
}

/// Tier B: the GPU words are the native ones, bit for bit.
fn check_tier_b(native: &[u32], gpu: &[u32]) {
    if let Some(i) = first_mismatch(native, gpu) {
        panic!(
            "Tier B: GPU word {i} differs from native ({:#010x} vs {:#010x})",
            gpu.get(i).copied().unwrap_or(0),
            native.get(i).copied().unwrap_or(0)
        );
    }
}

/// Acceptance `toolchain_trivial_kernel`, from payload §2 and Tier B: over 2¹⁶ edge and pseudo-random words, and over
/// lengths off the workgroup size of 64, the native build and the GPU build each write payload §2's pack/unpack, and
/// agree bit for bit.
#[test]
fn qa_toolchain_native_and_gpu_are_payload_s2_bit_for_bit() {
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    eprintln!("{}", h.adapter_info());
    let wgsl = kernel_wgsl();
    for len in [1usize << 16, 1, 2, 63, 65, 1000] {
        let input = qa_words(len);
        let n = native(&input);
        check_spec("native", &input, &n);
        let g = h.run_wgsl(&wgsl, ENTRY, &[&input]);
        check_spec("GPU", &input, &g);
        check_tier_b(&n, &g);
    }
}

negative_control!(
    qa_toolchain_native_and_gpu_are_payload_s2_bit_for_bit,
    "the input words themselves, where `packed_a`'s bits 10–31 must come back zero",
    expected = "is not payload §2's pack/unpack",
    {
        let input = qa_words(1 << 16);
        check_spec("GPU", &input, &input)
    }
);

/// Each Tier B field of the compiled kernel, as naga's WGSL reads it: the field, the read in the WGSL, a mis-built
/// read (its offset shifted by one bit, or its mask losing its low bit), the field's bits in the output word, and
/// whether it sits in `packed_a` (even index) or `times` (odd index).
const FIELDS: [(&str, &str, &str, u32, bool); 7] = [
    ("state", "& 7u) & 7u)", "& 6u) & 7u)", 0x7, true),
    (
        "detail",
        ">> bitcast<u32>(3u)) & 3u)",
        ">> bitcast<u32>(4u)) & 3u)",
        0x18,
        true,
    ),
    (
        "saturated",
        ">> bitcast<u32>(5u)) & 1u)",
        ">> bitcast<u32>(4u)) & 1u)",
        0x20,
        true,
    ),
    (
        "dmin_pair",
        ">> bitcast<u32>(6u)) & 3u)",
        ">> bitcast<u32>(7u)) & 3u)",
        0xc0,
        true,
    ),
    (
        "last_symbol",
        ">> bitcast<u32>(8u)) & 3u)",
        ">> bitcast<u32>(9u)) & 3u)",
        0x300,
        true,
    ),
    (
        "t_end_step",
        "& 65535u) & 65535u) & 65535u)",
        "& 65534u) & 65535u) & 65535u)",
        0xffff,
        false,
    ),
    (
        "t_dmin_step",
        ">> bitcast<u32>(16u)) & 65535u)",
        ">> bitcast<u32>(17u)) & 65535u)",
        0xffff_0000,
        false,
    ),
];

/// `wgsl` with `read` replaced by `bad`; panics unless the WGSL holds exactly one `read`.
fn misbuild(wgsl: &str, field: &str, read: &str, bad: &str) -> String {
    assert_eq!(
        wgsl.matches(read).count(),
        1,
        "naga's WGSL has no single read of `{field}` (`{read}`) to mis-build"
    );
    wgsl.replacen(read, bad, 1)
}

/// The mis-built GPU words `bad` differ from `native` somewhere, and only within `field`'s bits of the word it lives
/// in: the comparison sees a one-bit change of each field's offset, and pins it to that field.
fn check_seen(field: &str, bits: u32, in_packed_a: bool, native: &[u32], bad: &[u32]) {
    let mut seen = 0usize;
    for (i, (&n, &b)) in native.iter().zip(bad).enumerate() {
        let diff = n ^ b;
        if diff == 0 {
            continue;
        }
        seen += 1;
        assert_eq!(
            (i.is_multiple_of(2), diff & !bits),
            (in_packed_a, 0),
            "the `{field}` mis-build changed word {i} outside `{field}`'s bits ({diff:#010x})"
        );
    }
    assert!(
        seen > 0,
        "a mis-build of `{field}` went unseen by the bit-for-bit comparison"
    );
}

/// Tier B covers "all packed descriptor fields … and their bit offsets" and both step indices: a GPU build with any
/// one field's read mis-built differs from the native words, at that field's bits only.
#[test]
fn qa_toolchain_each_tier_b_field_misbuild_is_seen() {
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let wgsl = kernel_wgsl();
    let input = qa_words(1 << 16);
    let n = native(&input);
    check_tier_b(&n, &h.run_wgsl(&wgsl, ENTRY, &[&input]));
    for (field, read, bad, bits, in_a) in FIELDS {
        let g = h.run_wgsl(&misbuild(&wgsl, field, read, bad), ENTRY, &[&input]);
        check_seen(field, bits, in_a, &n, &g);
    }
}

negative_control!(
    qa_toolchain_each_tier_b_field_misbuild_is_seen,
    "a `mis-build` that changes nothing, which the comparison must not claim to see",
    expected = "went unseen by the bit-for-bit comparison",
    {
        let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
        let wgsl = kernel_wgsl();
        let input = qa_words(1 << 16);
        let (field, read, _, bits, in_a) = FIELDS[6];
        let g = h.run_wgsl(&misbuild(&wgsl, field, read, read), ENTRY, &[&input]);
        check_seen(field, bits, in_a, &native(&input), &g)
    }
);
