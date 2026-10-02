//! The codegen self-test (debug_tooling_plan §H; dd_generation_root §5 tests 1–4 and 9): pack∘unpack is the identity
//! per field, fuzzed over each field's full bit range with its top bit set, in three places — the host Rust, the
//! kernel's own pack/unpack on the GPU (TASK-M0-14's rust-gpu entry, `target/spirv/kernel.wgsl`) and the generated
//! WGSL fragment unpack on the GPU, through `codegen_selftest.wgsl`'s entry point, on a device requested with no
//! optional features, so no `shader-f16` (payload §1, §6). `times`' steps round-trip exactly and bit-identically
//! CPU/GPU; f16 pairs round-trip within f16 eps; `detail` decodes per `state`; the static layout check passes. Every
//! comparison is of whole values, never masked, and its control shows it can fail (pitfalls §9). CI runs it on
//! `macos-15` (Metal) and lavapipe on every commit (parity_contract §6; R-110, R-186).

use std::sync::OnceLock;

use kernel::payload::*;
use kernel::toolchain::{pack_unpack, UVec3};
use ledger::schema::{Entry, Ledger, Location};
use proptest::collection::vec;
use proptest::prelude::*;
use validation::gpu::{first_mismatch, GpuHarness};
use validation::{negative_control, prop};

/// The generated fragment unpack layer, and the self-test entry point appended to it.
const GENERATED: &str = include_str!("../../render/frag/generated/payload_unpack.wgsl");
const ENTRY_WGSL: &str = include_str!("codegen_selftest.wgsl");
const ENTRY: &str = "codegen_selftest_unpack";
/// The WGSL entry point naga names for `kernel::toolchain::pack_unpack`.
const KERNEL_ENTRY: &str = "toolchain_pack_unpack";

/// The packed fields, each at its selector in `codegen_selftest.wgsl`: payload §2's and the word buffer's `length`.
const FIELDS: [&str; 11] = [
    "state",
    "detail",
    "saturated",
    "dmin_pair",
    "last_symbol",
    "d_min",
    "dE_max",
    "dLz_max",
    "t_end_step",
    "t_dmin_step",
    "length",
];
const F16: [&str; 3] = ["d_min", "dE_max", "dLz_max"];
const T_END_FRACTION: u32 = 11;
const T_DMIN_FRACTION: u32 = 12;
/// Samples per property case: one GPU dispatch each.
const BATCH: usize = 64;

fn sel(name: &str) -> usize {
    FIELDS
        .iter()
        .position(|f| *f == name)
        .expect("a packed field")
}

/// The layout table's entries (dd_generation_root §3).
fn entries() -> &'static [Entry] {
    static ENTRIES: OnceLock<Vec<Entry>> = OnceLock::new();
    ENTRIES.get_or_init(|| ledger::gen::validate(&ledger::layout()).expect("the ledger validates"))
}

/// Field `name`'s word, offset and width, from the layout table.
fn location(name: &str) -> (&'static str, u32, u32) {
    match entries()
        .iter()
        .find(|e| e.name == name)
        .map(|e| &e.location)
    {
        Some(&Location::Packed {
            word,
            offset,
            width,
        }) => (word, offset, width),
        other => panic!("`{name}` is not a packed field: {other:?}"),
    }
}

/// A sample's packed words, each by the generated packer (`fgw_w`'s `length` by the generic insert, at the ledger's
/// bits: the march writes it, and no setter is generated). f16 fields hold their binary16 bits.
fn pack(s: &[u32]) -> [(&'static str, u32); 4] {
    let f = |n| s[sel(n)];
    let half = |n| f16_bits_to_f32(f(n) as u16);
    let (_, offset, width) = location("length");
    let counters = DminCounters::new();
    let (state, detail, sat) = (f("state"), f("detail"), f("saturated") == 1);
    let a = pack_packed_a(
        state,
        detail,
        sat,
        f("dmin_pair"),
        f("last_symbol"),
        half("d_min"),
        &counters,
    );
    [
        ("packed_a", a),
        ("packed_b", pack_packed_b(half("dE_max"), half("dLz_max"))),
        ("times", pack_times(f("t_end_step"), f("t_dmin_step"))),
        ("fgw_w", insert(0, f("length"), offset, width)),
    ]
}

fn word_of(words: &[(&str, u32)], name: &str) -> u32 {
    let (word, ..) = location(name);
    words.iter().find(|w| w.0 == word).expect("a packed word").1
}

/// What an unpack of field `name` must return for the packed value `raw`: u-bits as packed, an f16 field the bits of
/// its f32.
fn want(name: &str, raw: u32) -> u32 {
    if F16.contains(&name) {
        f16_bits_to_f32(raw as u16).to_bits()
    } else {
        raw
    }
}

/// The generated Rust unpack (payload §6), f32s as their bits.
fn host_unpack(name: &str, w: u32) -> u32 {
    match name {
        "state" => sd_state(w),
        "detail" => sd_detail(w),
        "saturated" => u32::from(sd_saturated(w)),
        "dmin_pair" => sd_dmin_pair(w),
        "last_symbol" => sd_last_symbol(w),
        "d_min" => pa_d_min(w).to_bits(),
        "dE_max" => pb_dE_max(w).to_bits(),
        "dLz_max" => pb_dLz_max(w).to_bits(),
        "t_end_step" => tm_t_end_step(w),
        "t_dmin_step" => tm_t_dmin_step(w),
        "length" => fgw_length_raw([0, 0, 0, w]),
        _ => unreachable!("{name}"),
    }
}

/// A field's values: its whole bit range, half the draws with its top bit set. `d_min` over (0, 65504] and its unset
/// +∞ (R-271; a negative `d_min` is refused, R-281); `dE_max`/`dLz_max` over every finite f16, sign bit included.
fn field_value(name: &str) -> BoxedStrategy<u32> {
    match name {
        "d_min" => prop_oneof![1u32..0x7c00, Just(PA_D_MIN_UNSET)].boxed(),
        "dE_max" | "dLz_max" => prop_oneof![0u32..0x7c00, 0x8000u32..0xfc00].boxed(),
        _ => {
            let top = 1u32 << (location(name).2 - 1);
            prop_oneof![0..top << 1, top..top << 1].boxed()
        }
    }
}

fn sample() -> Vec<BoxedStrategy<u32>> {
    FIELDS.iter().map(|n| field_value(n)).collect()
}

#[cfg(feature = "controls")]
/// A fixed batch drawn from the same strategy, for the controls.
#[cfg(feature = "controls")]
fn fixed_batch() -> Vec<Vec<u32>> {
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;
    let mut runner = TestRunner::deterministic();
    let strategy = vec(sample(), BATCH);
    strategy.new_tree(&mut runner).expect("a batch").current()
}

fn harness() -> GpuHarness {
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    eprintln!("{}", h.adapter_info());
    h
}

/// The generated layer with the self-test entry appended.
fn module(generated: &str) -> String {
    format!("{generated}\n{ENTRY_WGSL}")
}

/// The generated layer with `from` replaced by `to`, which must occur exactly once (a control's mis-generation).
#[cfg(feature = "controls")]
fn altered(from: &str, to: &str) -> String {
    assert_eq!(
        GENERATED.matches(from).count(),
        1,
        "the generated WGSL has no single `{from}`"
    );
    module(&GENERATED.replacen(from, to, 1))
}

/// The generated layer built with the sign-extending i32 `extractBits` overload in every accessor.
fn i32_module() -> String {
    assert!(GENERATED.contains("extractBits("));
    let helper = "fn st_xb_i32(e: u32, o: u32, c: u32) -> u32 { return bitcast<u32>(extractBits(bitcast<i32>(e), o, c)); }";
    format!(
        "{}\n{helper}",
        module(&GENERATED.replace("extractBits(", "st_xb_i32("))
    )
}

/// The self-test entry over `(word, selector, arg)` jobs, one invocation each.
fn wgsl_unpack(h: &GpuHarness, module: &str, jobs: &[(u32, u32, u32)]) -> Vec<u32> {
    let (mut w, mut s, mut a) = (Vec::new(), Vec::new(), Vec::new());
    for &(x, y, z) in jobs {
        w.push(x);
        s.push(y);
        a.push(z);
    }
    h.run_wgsl(module, ENTRY, &[&w, &s, &a])
}

/// Field `name` of `sample` was unpacked as `got` on `path`.
fn check_field(path: &str, name: &str, raw: u32, got: u32) {
    let want = want(name, raw);
    assert!(
        got == want,
        "{path}: pack∘unpack is not the identity for `{name}`: packed {raw:#x}, unpacked {got:#x}, want {want:#x}"
    );
}

/// The host path: each field unpacked from its word, `observe` applied to the words first, is the value packed, and the
/// fields repacked are the observed words, bit for bit — a field-only comparison masks the bits no accessor reads
/// (pitfalls §9).
fn check_host(batch: &[Vec<u32>], observe: impl Fn(&str, u32) -> u32) {
    for s in batch {
        let words: Vec<(&str, u32)> = pack(s).iter().map(|&(n, w)| (n, observe(n, w))).collect();
        let got: Vec<u32> = FIELDS
            .iter()
            .map(|n| host_unpack(n, word_of(&words, n)))
            .collect();
        for (k, name) in FIELDS.iter().enumerate() {
            check_field("host Rust", name, s[k], got[k]);
        }
        let raw: Vec<u32> = (FIELDS.iter().zip(&got))
            .map(|(n, &g)| {
                if F16.contains(n) {
                    f32_to_f16_bits(f32::from_bits(g)).into()
                } else {
                    g
                }
            })
            .collect();
        assert_eq!(
            pack(&raw).to_vec(),
            words,
            "host Rust: the repacked words are not the observed words"
        );
    }
}

/// The WGSL path: every field of every sample unpacked on the GPU by `module`.
fn check_wgsl(h: &GpuHarness, module: &str, batch: &[Vec<u32>]) {
    let jobs: Vec<(u32, u32, u32)> = (batch.iter())
        .flat_map(|s| {
            let words = pack(s);
            (0..FIELDS.len()).map(move |k| (word_of(&words, FIELDS[k]), k as u32, 0))
        })
        .collect();
    let got = wgsl_unpack(h, module, &jobs);
    for (s, got) in batch.iter().zip(got.chunks(FIELDS.len())) {
        for (k, name) in FIELDS.iter().enumerate() {
            check_field("WGSL unpack on the GPU", name, s[k], got[k]);
        }
    }
}

/// The WGSL `cargo xtask build-kernel` wrote.
fn kernel_wgsl() -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/spirv/kernel.wgsl"
    );
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{path}: {e}; run `cargo xtask build-kernel` first"))
}

/// The kernel build with its read of `detail` (bits 3–4) shifted to bit 4.
#[cfg(feature = "controls")]
fn misbuilt_kernel() -> String {
    let (wgsl, read) = (kernel_wgsl(), ">> bitcast<u32>(3u)) & 3u)");
    assert_eq!(
        wgsl.matches(read).count(),
        1,
        "the kernel WGSL has no single read of `detail`"
    );
    wgsl.replacen(read, ">> bitcast<u32>(4u)) & 3u)", 1)
}

/// The kernel path: each sample's `packed_a` at an even index and `times` at an odd one, as the kernel reads them; its
/// output bit-identical to the native kernel's, and its fields the values packed. The kernel packs the descriptor and
/// `times`, not `d_min` (TASK-M0-14).
fn check_kernel(h: &GpuHarness, wgsl: &str, batch: &[Vec<u32>]) {
    let input: Vec<u32> = (batch.iter())
        .flat_map(|s| [word_of(&pack(s), "state"), word_of(&pack(s), "t_end_step")])
        .collect();
    let gpu = h.run_wgsl(wgsl, KERNEL_ENTRY, &[&input]);
    for (s, out) in batch.iter().zip(gpu.chunks(2)) {
        for (k, name) in FIELDS.iter().enumerate().filter(|(_, n)| !F16.contains(n)) {
            let word = match location(name).0 {
                "packed_a" => out[0],
                "times" => out[1],
                _ => continue,
            };
            check_field("kernel on the GPU", name, s[k], host_unpack(name, word));
        }
    }
    let mut native = vec![0; input.len()];
    for i in 0..input.len() as u32 {
        pack_unpack(UVec3::new(i, 0, 0), &input, &mut native);
    }
    let i = first_mismatch(&native, &gpu);
    assert!(
        i.is_none(),
        "kernel on the GPU differs from the native kernel at word {i:?}"
    );
}

#[test]
fn codegen_selftest_pack_unpack_host() {
    prop::run(&vec(sample(), BATCH), |batch| {
        check_host(&batch, |_, w| w);
        Ok(())
    });
}

negative_control!(
    codegen_selftest_pack_unpack_host,
    "a contaminated reserved bit (packed_a bit 10), which no accessor reads, must fail the raw-word comparison",
    expected = "host Rust: the repacked words are not the observed words",
    check_host(&fixed_batch(), |n, w| if n == "packed_a" { w | 1 << 10 } else { w })
);

#[test]
fn codegen_selftest_pack_unpack_wgsl_gpu() {
    let (h, module) = (harness(), module(GENERATED));
    prop::run(&vec(sample(), BATCH), |batch| {
        check_wgsl(&h, &module, &batch);
        Ok(())
    });
}

negative_control!(
    codegen_selftest_pack_unpack_wgsl_gpu,
    "a WGSL accessor with a shifted offset (`detail` read at bit 4) must fail the round trip",
    expected = "WGSL unpack on the GPU: pack∘unpack is not the identity for `detail`",
    check_wgsl(
        &harness(),
        &altered("extractBits(w, 3u, 2u)", "extractBits(w, 4u, 2u)"),
        &fixed_batch()
    )
);

#[test]
fn codegen_selftest_pack_unpack_kernel_gpu() {
    let (h, wgsl) = (harness(), kernel_wgsl());
    prop::run(&vec(sample(), BATCH), |batch| {
        check_kernel(&h, &wgsl, &batch);
        Ok(())
    });
}

negative_control!(
    codegen_selftest_pack_unpack_kernel_gpu,
    "the kernel built with `detail`'s offset shifted must fail the round trip",
    expected = "kernel on the GPU: pack∘unpack is not the identity for `detail`",
    check_kernel(&harness(), &misbuilt_kernel(), &fixed_batch())
);

/// `times` over every u16 in each half: exact on the host, and the WGSL unpack and the kernel on the GPU bit-identical
/// to it; the display fraction exactly 0 and 1 at the endpoints, for every `horizon_steps` (R-86).
fn check_times(h: &GpuHarness, module: &str, kernel: &str) {
    let words: Vec<u32> = (0..1u32 << 16)
        .map(|i| pack_times(i, i.wrapping_mul(40503) & 0xffff))
        .collect();
    for (i, &w) in words.iter().enumerate() {
        let want = (i as u32, (i as u32).wrapping_mul(40503) & 0xffff);
        assert_eq!(
            (tm_t_end_step(w), tm_t_dmin_step(w)),
            want,
            "times: host round trip is not exact"
        );
    }
    let host: Vec<u32> = words
        .iter()
        .flat_map(|&w| [tm_t_end_step(w), tm_t_dmin_step(w)])
        .collect();
    let (end, dmin) = (sel("t_end_step") as u32, sel("t_dmin_step") as u32);
    let jobs: Vec<_> = words
        .iter()
        .flat_map(|&w| [(w, end, 0), (w, dmin, 0)])
        .collect();
    let i = first_mismatch(&host, &wgsl_unpack(h, module, &jobs));
    assert!(
        i.is_none(),
        "times: the WGSL unpack on the GPU differs from the host at {i:?}"
    );
    let input: Vec<u32> = words.iter().flat_map(|&w| [0, w]).collect();
    let out = h.run_wgsl(kernel, KERNEL_ENTRY, &[&input]);
    let i = first_mismatch(&input, &out);
    assert!(
        i.is_none(),
        "times: the kernel on the GPU is not the identity at {i:?}"
    );
    let (one, zero) = (1f32.to_bits(), 0f32.to_bits());
    let mut jobs = Vec::new();
    for n in 1..=u16::MAX as u32 {
        let (at_end, at_start) = (pack_times(n, 0), pack_times(0, n));
        let host = [
            tm_t_end_fraction(at_end, n),
            tm_t_dmin_fraction(at_end, n),
            tm_t_end_fraction(at_start, n),
        ];
        assert_eq!(
            host.map(f32::to_bits),
            [one, zero, zero],
            "times: host fraction endpoints at {n}"
        );
        assert_eq!(
            tm_t_dmin_fraction(at_start, n).to_bits(),
            one,
            "times: host fraction endpoints at {n}"
        );
        jobs.extend([(at_end, T_END_FRACTION, n), (at_end, T_DMIN_FRACTION, n)]);
        jobs.extend([
            (at_start, T_END_FRACTION, n),
            (at_start, T_DMIN_FRACTION, n),
        ]);
    }
    let want: Vec<u32> = (1..=u16::MAX)
        .flat_map(|_| [one, zero, zero, one])
        .collect();
    let i = first_mismatch(&want, &wgsl_unpack(h, module, &jobs));
    assert!(
        i.is_none(),
        "times: a GPU fraction endpoint is not exactly 0 or 1 at job {i:?}"
    );
    check_zero_horizon(h, module);
}

/// At `horizon_steps = 0` both display fractions are exactly 0, on the host and on the GPU, whatever the steps: the
/// `horizon_steps > 0u` guard is outermost, so `s == horizon_steps` at 0 does not give 1 (payload §6; R-365).
fn check_zero_horizon(h: &GpuHarness, module: &str) {
    let words = [
        pack_times(0, 0),
        pack_times(1, 0xffff),
        pack_times(0xffff, 1),
        pack_times(0x1234, 0x5678),
    ];
    let zero = 0f32.to_bits();
    for &w in &words {
        assert_eq!(
            [tm_t_end_fraction(w, 0), tm_t_dmin_fraction(w, 0)].map(f32::to_bits),
            [zero, zero],
            "times: a host fraction at horizon_steps = 0 is not exactly 0 for {w:#010x}"
        );
    }
    let jobs: Vec<_> = (words.iter())
        .flat_map(|&w| [(w, T_END_FRACTION, 0), (w, T_DMIN_FRACTION, 0)])
        .collect();
    let i = first_mismatch(&vec![zero; jobs.len()], &wgsl_unpack(h, module, &jobs));
    assert!(
        i.is_none(),
        "times: a GPU fraction at horizon_steps = 0 is not exactly 0 at job {i:?}"
    );
}

#[test]
fn codegen_selftest_times() {
    check_times(&harness(), &module(GENERATED), &kernel_wgsl());
}

#[test]
fn codegen_selftest_times_zero_horizon() {
    check_zero_horizon(&harness(), &module(GENERATED));
}

negative_control!(
    codegen_selftest_times_zero_horizon,
    "the guard nested inside the endpoint select must give 1.0, not 0, at `pack_times(0, 0)` with `horizon_steps = 0`",
    expected = "times: a GPU fraction at horizon_steps = 0 is not exactly 0",
    check_zero_horizon(
        &harness(),
        &altered(
            "tm_t_end_step(w);\n    return select(0.0, select(f32(s) / f32(horizon_steps), 1.0, s == horizon_steps), horizon_steps > 0u);",
            "tm_t_end_step(w);\n    return select(select(0.0, f32(s) / f32(horizon_steps), horizon_steps > 0u), 1.0, s == horizon_steps);"
        )
    )
);

negative_control!(
    codegen_selftest_times,
    "a WGSL `t_dmin_step` accessor with a shifted offset must differ from the host",
    expected = "times: the WGSL unpack on the GPU differs from the host",
    check_times(
        &harness(),
        &altered(
            "tm_t_dmin_step(w: u32) -> u32 { return extractBits(w, 16u",
            "tm_t_dmin_step(w: u32) -> u32 { return extractBits(w, 15u"
        ),
        &kernel_wgsl()
    )
);

/// The static layout check passes on the layout table (dd_generation_root §5 test 1), and the generated Rust's reserved
/// spans are the table's.
fn check_static(layout: &Ledger) {
    let entries = ledger::gen::validate(layout).expect("the ledger validates");
    let found = ledger::check::check(&layout.words, &entries);
    assert!(found.is_empty(), "static layout check failed: {found:?}");
    for word in &layout.words {
        let generated: &[(u32, u32)] = match word.name {
            "packed_a" => &PACKED_A_RESERVED,
            "packed_b" => &PACKED_B_RESERVED,
            "times" => &TIMES_RESERVED,
            _ => continue,
        };
        let table: Vec<(u32, u32)> = word.reserved.iter().map(|s| (s.offset, s.width)).collect();
        assert_eq!(
            generated,
            &table[..],
            "static layout check failed: `{}`'s reserved spans",
            word.name
        );
    }
}

#[test]
fn codegen_selftest_static_layout() {
    check_static(&ledger::layout());
}

negative_control!(
    codegen_selftest_static_layout,
    "`detail` moved onto `state`'s bits must fail the overlap check",
    expected = "static layout check failed",
    {
        let mut layout = ledger::layout();
        let detail = layout
            .entries
            .iter_mut()
            .find(|e| e.name == Some("detail"))
            .expect("detail");
        detail.location = Some(Location::Packed {
            word: "packed_a",
            offset: 2,
            width: 2,
        });
        check_static(&layout)
    }
);

/// Whether `got` is within f16 eps (2⁻¹⁰, relative; absolute below the least normal 2⁻¹⁴) of `x`.
fn within_f16_eps(x: f32, got: f32) -> bool {
    (x - got).abs() <= 2f32.powi(-10) * x.abs().max(2f32.powi(-14))
}

/// f16 pairs: each half packed by the Rust `pack2x16float` and read back by the generated `.x`/`.y` accessors, on the
/// host and on the GPU, is within f16 eps of the value, and the GPU's read is the host's bit for bit.
fn check_f16_pairs(h: &GpuHarness, module: &str, pairs: &[(f32, f32)]) {
    let fields = [("dE_max", 0), ("dLz_max", 1), ("d_min", 1)];
    let words: Vec<u32> = pairs.iter().map(|&(a, b)| pack2x16float([a, b])).collect();
    let jobs: Vec<_> = (words.iter())
        .flat_map(|&w| fields.map(|(n, _)| (w, sel(n) as u32, 0)))
        .collect();
    let gpu = wgsl_unpack(h, module, &jobs);
    for ((&(a, b), &w), gpu) in pairs.iter().zip(&words).zip(gpu.chunks(3)) {
        for (&(name, half), &g) in fields.iter().zip(gpu) {
            let (x, got) = ([a, b][half], f32::from_bits(g));
            assert!(
                within_f16_eps(x, got),
                "f16 pair: `{name}` on the GPU is {got}, not within f16 eps of {x}"
            );
            assert_eq!(
                g,
                host_unpack(name, w),
                "f16 pair: `{name}` on the GPU is not the host's"
            );
        }
    }
}

fn f16_value() -> impl Strategy<Value = f32> {
    prop_oneof![-65504f32..=65504f32, -6.2e-5f32..=6.2e-5f32]
}

#[test]
fn codegen_selftest_f16_pairs() {
    let (h, module) = (harness(), module(GENERATED));
    prop::run(&vec((f16_value(), f16_value()), BATCH), |pairs| {
        check_f16_pairs(&h, &module, &pairs);
        Ok(())
    });
}

negative_control!(
    codegen_selftest_f16_pairs,
    "a WGSL `dE_max` accessor reading the high half must fail the round trip",
    expected = "f16 pair: `dE_max` on the GPU is",
    check_f16_pairs(
        &harness(),
        &altered("unpack2x16float(w).x", "unpack2x16float(w).y"),
        &[(1.0, 2.0), (-3.5, 0.25)]
    )
);

/// `detail`'s legend, keyed by `state` (debug_tooling_plan §B; payload §2): the ledger's meanings for the four states
/// it is defined in, none for bounded, running and the reserved codes.
fn legend(state: u32, detail: u32) -> Option<&'static str> {
    let name = ledger::payload::states().get(state as usize).copied()?;
    let meanings = ledger::payload::detail_meanings();
    meanings
        .iter()
        .find(|(s, _)| *s == name)
        .map(|(_, m)| m[detail as usize])
}

/// Every `(state, detail)`, its neighbours' bits all set, decoded on the GPU: both read back, and the legend switches on
/// the state read — a body under escape, a pair under collision, the failure category under the failures, nothing
/// under bounded or running (the three-colours bug: `detail` read, not dropped).
fn check_detail(h: &GpuHarness, module: &str) {
    let combos: Vec<(u32, u32)> = (0..8).flat_map(|s| (0..4).map(move |d| (s, d))).collect();
    let words: Vec<u32> = (combos.iter())
        .map(|&(s, d)| pack_packed_a(s, d, true, 3, 3, f32::INFINITY, &DminCounters::new()))
        .collect();
    let (state, detail) = (sel("state") as u32, sel("detail") as u32);
    let jobs: Vec<_> = words
        .iter()
        .flat_map(|&w| [(w, state, 0), (w, detail, 0)])
        .collect();
    let got = wgsl_unpack(h, module, &jobs);
    for (&(s, d), got) in combos.iter().zip(got.chunks(2)) {
        assert_eq!(
            (got[0], got[1]),
            (s, d),
            "detail per state: packed (state, detail) read back wrong"
        );
        let want = match s {
            0 => Some(if d < 3 {
                format!("body {d}")
            } else {
                "all three: triple ejection".into()
            }),
            2 => Some(if d < 3 {
                format!("pair {d}")
            } else {
                "all three: triple collision".into()
            }),
            4 | 5 => {
                Some(ledger::payload::detail_meanings()[s as usize - 2].1[d as usize].to_owned())
            }
            _ => None,
        };
        assert_eq!(
            legend(got[0], got[1]).map(str::to_owned),
            want,
            "detail per state: legend for ({s}, {d})"
        );
    }
}

#[test]
fn codegen_selftest_detail_per_state() {
    check_detail(&harness(), &module(GENERATED));
}

negative_control!(
    codegen_selftest_detail_per_state,
    "a WGSL `detail` accessor with a shifted offset must fail the per-state decode",
    expected = "detail per state: packed (state, detail) read back wrong",
    check_detail(
        &harness(),
        &altered("extractBits(w, 3u, 2u)", "extractBits(w, 4u, 2u)")
    )
);

/// Every value of each u-bits field with its top bit set, every other bit of its word set too, read by `module` on the
/// GPU: the value, zero-extended (REQ-TOOL-003).
fn check_top_bit(h: &GpuHarness, module: &str) {
    let mut jobs = Vec::new();
    for (k, name) in FIELDS.iter().enumerate().filter(|(_, n)| !F16.contains(n)) {
        let (_, offset, width) = location(name);
        for v in 1u32 << (width - 1)..1u32 << width {
            jobs.push((insert(u32::MAX, v, offset, width), k as u32, v));
        }
    }
    let got = wgsl_unpack(h, module, &jobs);
    for (&(w, k, v), &g) in jobs.iter().zip(&got) {
        let name = FIELDS[k as usize];
        assert!(
            g == v,
            "`{name}` with its top bit set read {g:#x} on the GPU, not {v:#x} (word {w:#010x})"
        );
    }
}

#[test]
fn codegen_selftest_extractbits_top_bit() {
    let h = harness();
    check_top_bit(&h, &module(GENERATED));
    let i32_build = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        check_top_bit(&h, &i32_module())
    }));
    assert!(
        i32_build.is_err(),
        "the i32 extractBits build passed the top-bit check"
    );
}

negative_control!(
    codegen_selftest_extractbits_top_bit,
    "the accessors built with the i32 extractBits overload sign-extend",
    expected = "with its top bit set read 0xffff",
    check_top_bit(&harness(), &i32_module())
);

/// Every binary16 pattern in each half of a word, read by the WGSL f16 accessors on the GPU: the Rust unpack's f32, bit
/// for bit (a NaN matches a NaN).
fn check_f16_unpack(h: &GpuHarness, module: &str) {
    let words: Vec<u32> = (0..1u32 << 16)
        .map(|i| i | (i.wrapping_mul(40503) & 0xffff) << 16)
        .collect();
    let jobs: Vec<_> = (words.iter())
        .flat_map(|&w| F16.map(|n| (w, sel(n) as u32, 0)))
        .collect();
    for (&(w, k, _), &g) in jobs.iter().zip(&wgsl_unpack(h, module, &jobs)) {
        let (name, (gf, r)) = (
            FIELDS[k as usize],
            (f32::from_bits(g), host_unpack(FIELDS[k as usize], w)),
        );
        assert!(
            g == r || (gf.is_nan() && f32::from_bits(r).is_nan()),
            "`{name}` unpacked without shader-f16 is not the Rust unpack: word {w:#010x}, GPU {g:#010x}, Rust {r:#010x}"
        );
    }
}

#[test]
fn codegen_selftest_unpack_without_shader_f16() {
    let h = harness();
    assert!(
        !h.features().contains(wgpu::Features::SHADER_F16),
        "the device was requested with shader-f16"
    );
    let enables = GENERATED
        .lines()
        .any(|l| l.trim_start().starts_with("enable"));
    assert!(!enables, "the generated unpack has an `enable` directive");
    check_f16_unpack(&h, &module(GENERATED));
}

negative_control!(
    codegen_selftest_unpack_without_shader_f16,
    "a WGSL `dE_max` accessor reading the high half must differ from the Rust unpack",
    expected = "unpacked without shader-f16 is not the Rust unpack",
    check_f16_unpack(
        &harness(),
        &altered("unpack2x16float(w).x", "unpack2x16float(w).y")
    )
);
