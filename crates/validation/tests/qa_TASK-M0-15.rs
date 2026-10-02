//! QA tests for TASK-M0-15, written from the requirements it closes (REQ-GEN-004, REQ-GEN-006, REQ-GEN-007,
//! REQ-TOOL-003, REQ-PAY-011), from REQ-RENDER-019's statement of the fraction's empty-horizon value, and from the
//! rulings R-361 and R-365. Expected values come from the corpus, not from the implementation: field offsets and widths
//! from dd_simstate_payload §2's bit tables and §3's `.w` bit map, f16 known answers from IEEE 754 binary16, and the
//! display fraction's endpoints from REQ-GEN-006 ("exactly 0 and 1 at the endpoints") and REQ-RENDER-019 ("returning 0
//! when `horizon_steps` is 0").
//!
//! Every GPU dispatch here runs the generated fragment layer, `crates/render/frag/generated/payload_unpack.wgsl`, with
//! a small entry point of this file's own; none needs the built kernel, so the file runs wherever a GPU adapter is
//! configured. The test names start `codegen_selftest_` so CI's Metal and lavapipe steps (`codegen_selftest` filter)
//! run them on both adapters (R-110, R-186). Each test registers a negative control (R-176).

use kernel::payload::{
    pa_d_min, pb_dE_max, pb_dLz_max, sd_detail, sd_dmin_pair, sd_last_symbol, sd_saturated,
    sd_state, tm_t_dmin_fraction, tm_t_dmin_step, tm_t_end_fraction, tm_t_end_step,
};
use proptest::prelude::*;
use validation::gpu::GpuHarness;
use validation::{negative_control, prop};

const GENERATED: &str = include_str!("../../render/frag/generated/payload_unpack.wgsl");

/// This file's entry point: `out[i]` is the accessor `sel[i]` names applied to `word[i]`, `arg[i]` the
/// `horizon_steps` of the fractions. f32 results as their bits; a bool as 0/1.
const ENTRY: &str = r"
@group(0) @binding(0) var<storage, read> qa_word: array<u32>;
@group(0) @binding(1) var<storage, read> qa_sel: array<u32>;
@group(0) @binding(2) var<storage, read> qa_arg: array<u32>;
@group(0) @binding(3) var<storage, read_write> qa_out: array<u32>;

fn qa_read(w: u32, s: u32, a: u32) -> u32 {
    switch s {
        case 0u: { return sd_state(w); }
        case 1u: { return sd_detail(w); }
        case 2u: { return select(0u, 1u, sd_saturated(w)); }
        case 3u: { return sd_dmin_pair(w); }
        case 4u: { return sd_last_symbol(w); }
        case 5u: { return tm_t_end_step(w); }
        case 6u: { return tm_t_dmin_step(w); }
        case 7u: { return fgw_length_raw(vec4<u32>(0xffffffffu, 0xffffffffu, 0xffffffffu, w)); }
        case 8u: { return bitcast<u32>(pa_d_min(w)); }
        case 9u: { return bitcast<u32>(pb_dE_max(w)); }
        case 10u: { return bitcast<u32>(pb_dLz_max(w)); }
        case 11u: { return bitcast<u32>(tm_t_end_fraction(w, a)); }
        case 12u: { return bitcast<u32>(tm_t_dmin_fraction(w, a)); }
        default: { return 0xdeadbeefu; }
    }
}

@compute @workgroup_size(64)
fn qa_m015(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x < arrayLength(&qa_word)) { qa_out[id.x] = qa_read(qa_word[id.x], qa_sel[id.x], qa_arg[id.x]); }
}
";

/// The u-bits fields at payload §2's and §3's offsets and widths, as `(name, selector, offset, width)`: the descriptor
/// in `packed_a` bits 0–9, `times`' two u16 halves, and the word buffer's `length` at `.w` bits 25–31.
const U_FIELDS: [(&str, u32, u32, u32); 8] = [
    ("state", 0, 0, 3),
    ("detail", 1, 3, 2),
    ("saturated", 2, 5, 1),
    ("dmin_pair", 3, 6, 2),
    ("last_symbol", 4, 8, 2),
    ("t_end_step", 5, 0, 16),
    ("t_dmin_step", 6, 16, 16),
    ("length", 7, 25, 7),
];
const SEL_D_MIN: u32 = 8;
const SEL_DE_MAX: u32 = 9;
const SEL_DLZ_MAX: u32 = 10;
const SEL_END_FRACTION: u32 = 11;
const SEL_DMIN_FRACTION: u32 = 12;

fn harness() -> GpuHarness {
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    eprintln!("{}", h.adapter_info());
    h
}

fn module(generated: &str) -> String {
    format!("{generated}\n{ENTRY}")
}

/// Runs `(word, selector, arg)` jobs through `generated` with this file's entry point.
fn run(h: &GpuHarness, generated: &str, jobs: &[(u32, u32, u32)]) -> Vec<u32> {
    let words: Vec<u32> = jobs.iter().map(|j| j.0).collect();
    let sels: Vec<u32> = jobs.iter().map(|j| j.1).collect();
    let args: Vec<u32> = jobs.iter().map(|j| j.2).collect();
    h.run_wgsl(&module(generated), "qa_m015", &[&words, &sels, &args])
}

/// A `times` word built by hand from payload §2: `t_end_step` in bits 0–15, `t_dmin_step` in bits 16–31.
fn times_word(t_end: u32, t_dmin: u32) -> u32 {
    t_end | t_dmin << 16
}

/// The generated layer with its two display-fraction functions replaced by `fractions` (a control's mis-generation).
#[cfg(feature = "controls")]
fn with_fractions(fractions: &str) -> String {
    let mut out = GENERATED.to_owned();
    for name in ["fn tm_t_end_fraction(", "fn tm_t_dmin_fraction("] {
        assert_eq!(out.matches(name).count(), 1, "no single `{name}`");
        let start = out.find(name).unwrap();
        let end = start + out[start..].find("\n}\n").expect("function end") + 3;
        out.replace_range(start..end, "");
    }
    format!("{out}\n{fractions}")
}

// ------------------------------------------------------------------------------------------------------------------
// The display fraction's endpoints (REQ-GEN-006; REQ-RENDER-019; R-361, R-365)
// ------------------------------------------------------------------------------------------------------------------

/// REQ-GEN-006: for every horizon `h` in 1..=65535, a step equal to `h` reads exactly 1.0 and a step of 0 exactly +0.0,
/// in each of `times`' halves, on the GPU through `generated` and on the host.
fn check_one_at_horizon(h: &GpuHarness, generated: &str) {
    let (one, zero) = (1f32.to_bits(), 0f32.to_bits());
    let mut jobs = Vec::new();
    let mut want = Vec::new();
    for n in 1..=u16::MAX as u32 {
        let (end_at_h, dmin_at_h) = (times_word(n, 0), times_word(0, n));
        jobs.extend([
            (end_at_h, SEL_END_FRACTION, n),
            (end_at_h, SEL_DMIN_FRACTION, n),
            (dmin_at_h, SEL_END_FRACTION, n),
            (dmin_at_h, SEL_DMIN_FRACTION, n),
        ]);
        want.extend([one, zero, zero, one]);
        let host = [
            tm_t_end_fraction(end_at_h, n),
            tm_t_dmin_fraction(end_at_h, n),
            tm_t_end_fraction(dmin_at_h, n),
            tm_t_dmin_fraction(dmin_at_h, n),
        ];
        assert_eq!(
            host.map(f32::to_bits),
            [one, zero, zero, one],
            "display fraction on the host: endpoints at horizon {n}"
        );
    }
    let got = run(h, generated, &jobs);
    let missed: Vec<(u32, u32, f32)> = (jobs.iter().zip(&want).zip(&got))
        .filter(|((_, w), g)| w != g)
        .map(|((j, _), &g)| (j.2, j.1, f32::from_bits(g)))
        .collect();
    assert!(
        missed.is_empty(),
        "display fraction on the GPU: {} endpoints are not exactly 0 or 1 (horizon, selector, value), first {:?}",
        missed.len(),
        &missed[..missed.len().min(8)]
    );
}

#[test]
fn codegen_selftest_qa_fraction_one_at_horizon() {
    check_one_at_horizon(&harness(), GENERATED);
}

// The fraction as it was generated before R-361. On Metal, `f32(n) / f32(n)` is not 1.0 for thousands of horizons, so
// this form fails there; the second form, the same division as the reciprocal multiply Metal compiles it to, misses
// 1.0 on every adapter (e.g. 41 · RN(1/41) ≠ 1 in f32), so the control fails on lavapipe as well.
negative_control!(
    codegen_selftest_qa_fraction_one_at_horizon,
    "the pre-R-361 fraction, which divides at the endpoint, must miss 1.0",
    expected = "display fraction on the GPU",
    {
        let h = harness();
        check_one_at_horizon(
            &h,
            &with_fractions(
                "fn tm_t_end_fraction(w: u32, horizon_steps: u32) -> f32 { return select(0.0, f32(tm_t_end_step(w)) / f32(horizon_steps), horizon_steps > 0u); }
fn tm_t_dmin_fraction(w: u32, horizon_steps: u32) -> f32 { return select(0.0, f32(tm_t_dmin_step(w)) / f32(horizon_steps), horizon_steps > 0u); }",
            ),
        );
        check_one_at_horizon(
            &h,
            &with_fractions(
                "fn tm_t_end_fraction(w: u32, horizon_steps: u32) -> f32 { return select(0.0, f32(tm_t_end_step(w)) * (1.0 / f32(horizon_steps)), horizon_steps > 0u); }
fn tm_t_dmin_fraction(w: u32, horizon_steps: u32) -> f32 { return select(0.0, f32(tm_t_dmin_step(w)) * (1.0 / f32(horizon_steps)), horizon_steps > 0u); }",
            ),
        );
    }
);

/// REQ-RENDER-019 and R-365: with `horizon_steps = 0` the fraction is 0 (exactly +0.0, not −0, not NaN, not 1) for
/// every step, the step 0 included (where `s == h`), on the GPU through `generated` and on the host.
fn check_empty_horizon(h: &GpuHarness, generated: &str) {
    let steps = [0u32, 1, 2, 0x7fff, 0x8000, 0xfffe, 0xffff];
    let mut jobs = Vec::new();
    for &a in &steps {
        for &b in &steps {
            let w = times_word(a, b);
            jobs.extend([(w, SEL_END_FRACTION, 0), (w, SEL_DMIN_FRACTION, 0)]);
            assert_eq!(
                [tm_t_end_fraction(w, 0), tm_t_dmin_fraction(w, 0)].map(f32::to_bits),
                [0, 0],
                "empty horizon on the host: steps ({a}, {b}) do not read +0.0"
            );
        }
    }
    let got = run(h, generated, &jobs);
    for (j, g) in jobs.iter().zip(&got) {
        assert!(
            *g == 0,
            "empty horizon on the GPU: word {:#010x}, selector {} reads {} ({g:#010x}), not +0.0",
            j.0,
            j.1,
            f32::from_bits(*g)
        );
    }
}

#[test]
fn codegen_selftest_qa_fraction_empty_horizon() {
    check_empty_horizon(&harness(), GENERATED);
}

// The other nesting, the endpoint select outermost: at `s == h == 0` it reads 1.0 (R-365 rules the guard outermost).
negative_control!(
    codegen_selftest_qa_fraction_empty_horizon,
    "the endpoint select outside the h > 0 guard must read 1.0 at s = h = 0",
    expected = "empty horizon on the GPU",
    check_empty_horizon(
        &harness(),
        &with_fractions(
            "fn tm_t_end_fraction(w: u32, horizon_steps: u32) -> f32 { let s = tm_t_end_step(w); return select(select(0.0, f32(s) / f32(horizon_steps), horizon_steps > 0u), 1.0, s == horizon_steps); }
fn tm_t_dmin_fraction(w: u32, horizon_steps: u32) -> f32 { let s = tm_t_dmin_step(w); return select(select(0.0, f32(s) / f32(horizon_steps), horizon_steps > 0u), 1.0, s == horizon_steps); }",
        )
    )
);

// ------------------------------------------------------------------------------------------------------------------
// The u-bits fields at the doc's offsets, top bits set (REQ-GEN-004, REQ-GEN-007, REQ-TOOL-003)
// ------------------------------------------------------------------------------------------------------------------

/// The host Rust accessor for U_FIELDS' selector `sel`.
fn host_u(sel: u32, w: u32) -> u32 {
    match sel {
        0 => sd_state(w),
        1 => sd_detail(w),
        2 => u32::from(sd_saturated(w)),
        3 => sd_dmin_pair(w),
        4 => sd_last_symbol(w),
        5 => tm_t_end_step(w),
        6 => tm_t_dmin_step(w),
        7 => kernel::payload::fgw_length_raw([u32::MAX, u32::MAX, u32::MAX, w]),
        _ => unreachable!(),
    }
}

/// Each `(background, values)` sample: every field's value written at its doc offset into `background`, the other bits
/// of the word left as they are; the GPU accessor and the host accessor each read back exactly the value.
fn check_doc_fields(h: &GpuHarness, generated: &str, samples: &[(u32, [u32; 8])]) {
    let mut jobs = Vec::new();
    for &(bg, values) in samples {
        for (&(_, sel, off, width), &v) in U_FIELDS.iter().zip(&values) {
            let mask = (((1u64 << width) - 1) as u32) << off;
            jobs.push(((bg & !mask) | (v << off), sel, v));
        }
    }
    let got = run(h, generated, &jobs);
    for (&(w, sel, v), &g) in jobs.iter().zip(&got) {
        let name = U_FIELDS[sel as usize].0;
        assert_eq!(
            host_u(sel, w),
            v,
            "field `{name}` on the host: word {w:#010x} does not read {v:#x}"
        );
        assert!(
            g == v,
            "field `{name}` on the GPU: word {w:#010x} reads {g:#x}, not {v:#x}"
        );
    }
}

/// A field value over its whole range, half the draws with its top bit set.
fn value(width: u32) -> BoxedStrategy<u32> {
    let top = 1u32 << (width - 1);
    prop_oneof![0..top << 1, top..top << 1].boxed()
}

fn sample() -> impl Strategy<Value = (u32, [u32; 8])> {
    let [a, b, c, d, e, f, g, i] = U_FIELDS.map(|(_, _, _, w)| value(w));
    (any::<u32>(), [a, b, c, d, e, f, g, i])
}

#[test]
fn codegen_selftest_qa_doc_fields_property() {
    let h = harness();
    prop::run(&proptest::collection::vec(sample(), 64), |batch| {
        check_doc_fields(&h, GENERATED, &batch);
        Ok(())
    });
}

negative_control!(
    codegen_selftest_qa_doc_fields_property,
    "a WGSL `detail` accessor reading one bit too high must fail the round trip",
    expected = "field `detail` on the GPU",
    check_doc_fields(
        &harness(),
        &GENERATED.replacen("extractBits(w, 3u, 2u)", "extractBits(w, 4u, 2u)", 1),
        &[
            (0, [5, 1, 0, 2, 3, 1, 2, 3]),
            (0x0f0f_0f0f, [3, 2, 1, 0, 1, 0xffff, 0x8000, 76])
        ]
    )
);

/// Every value with the top bit set, of every field, in a word whose other bits are all set: the i32 overload's
/// failure (sign extension) and a reader that strays outside the field's bits both show here.
fn top_bit_samples() -> Vec<(u32, [u32; 8])> {
    let mut out = Vec::new();
    for (k, &(_, _, _, width)) in U_FIELDS.iter().enumerate() {
        for v in 1u32 << (width - 1)..=((1u64 << width) - 1) as u32 {
            let mut values = U_FIELDS.map(|(_, _, _, w)| ((1u64 << w) - 1) as u32);
            values[k] = v;
            out.push((u32::MAX, values));
        }
    }
    out
}

#[test]
fn codegen_selftest_qa_doc_fields_top_bit() {
    check_doc_fields(&harness(), GENERATED, &top_bit_samples());
}

negative_control!(
    codegen_selftest_qa_doc_fields_top_bit,
    "the accessors built with the sign-extending i32 extractBits overload must fail",
    expected = "on the GPU: word",
    check_doc_fields(
        &harness(),
        &format!(
            "{}\nfn qa_xb_i32(e: u32, o: u32, c: u32) -> u32 {{ return bitcast<u32>(extractBits(bitcast<i32>(e), o, c)); }}",
            GENERATED.replace("extractBits(", "qa_xb_i32(")
        ),
        &top_bit_samples()
    )
);

// ------------------------------------------------------------------------------------------------------------------
// f16 pairs without shader-f16 (REQ-PAY-011, REQ-GEN-007)
// ------------------------------------------------------------------------------------------------------------------

/// IEEE 754 binary16 patterns and their exact f32 values: normal, the largest finite, the least normal and subnormal,
/// both zeros and +∞ (`d_min`'s unset value).
const F16_KNOWN: [(u16, f32); 10] = [
    (0x3c00, 1.0),
    (0xc000, -2.0),
    (0x3555, 0.333_251_95),
    (0x7bff, 65504.0),
    (0xfbff, -65504.0),
    (0x0400, 6.103_515_6e-5),
    (0x0001, 5.960_464_5e-8),
    (0x0000, 0.0),
    (0x8000, -0.0),
    (0x7c00, f32::INFINITY),
];

/// Each known pattern in the low half (`dE_max`) and the high half (`d_min`, `dLz_max`) of a word whose other half is a
/// different known pattern: each accessor reads the value exactly, on the GPU through `generated` and on the host.
fn check_f16_known(h: &GpuHarness, generated: &str) {
    let mut jobs = Vec::new();
    let mut want = Vec::new();
    for (i, &(lo, lo_v)) in F16_KNOWN.iter().enumerate() {
        let (hi, hi_v) = F16_KNOWN[(i + 3) % F16_KNOWN.len()];
        let w = u32::from(lo) | u32::from(hi) << 16;
        assert_eq!(
            [pb_dE_max(w), pa_d_min(w), pb_dLz_max(w)].map(f32::to_bits),
            [lo_v, hi_v, hi_v].map(f32::to_bits),
            "f16 on the host: word {w:#010x}"
        );
        jobs.extend([(w, SEL_DE_MAX, 0), (w, SEL_D_MIN, 0), (w, SEL_DLZ_MAX, 0)]);
        want.extend([(lo_v, "dE_max"), (hi_v, "d_min"), (hi_v, "dLz_max")]);
    }
    let got = run(h, generated, &jobs);
    for ((j, (v, name)), &g) in jobs.iter().zip(&want).zip(&got) {
        assert!(
            g == v.to_bits(),
            "f16 `{name}` on the GPU without shader-f16: word {:#010x} reads {} ({g:#010x}), not {v}",
            j.0,
            f32::from_bits(g)
        );
    }
}

#[test]
fn codegen_selftest_qa_f16_without_shader_f16() {
    let h = harness();
    assert!(
        !h.features().contains(wgpu::Features::SHADER_F16),
        "the self-test device has shader-f16"
    );
    assert!(
        !GENERATED
            .lines()
            .any(|l| l.split("//").next().unwrap_or("").contains("enable")),
        "the generated fragment layer has an `enable` directive"
    );
    check_f16_known(&h, GENERATED);
}

negative_control!(
    codegen_selftest_qa_f16_without_shader_f16,
    "`dE_max` read from the high half must miss the known answers",
    expected = "f16 `dE_max` on the GPU without shader-f16",
    check_f16_known(
        &harness(),
        &GENERATED.replacen("unpack2x16float(w).x", "unpack2x16float(w).y", 1)
    )
);
