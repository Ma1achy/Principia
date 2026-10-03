//! QA tests for TASK-M0-50, round 4, written from REQ-RENDER-083: a comparison against "an inf or NaN constant (a
//! constant expression that evaluates to one included)" and against a finite-max stand-in "as a `bitcast` of its bit
//! pattern or as another constant expression that evaluates to it" must fail the lint (R-351, R-352, R-353), in any
//! fragment-stage WGSL file, f16 and 16- and 64-bit integers included.
//!
//! Each value below is worked out by hand from the bit patterns, independently of the lint: integer arithmetic wraps at
//! the operands' width, a signed right shift copies the sign bit, a conversion between integer widths sign- or
//! zero-extends by the source's signedness and then truncates, a signed `max` or `<` orders by sign. The leaves are
//! `bitcast`s, which naga does not fold, so the lint's own evaluator computes each value. Each firing case has a near
//! miss one step away (the other signedness, one bit off) that evaluates to a finite value other than a stand-in and
//! must stay quiet. Each test has a registered negative control (R-176).

use validation::negative_control;
use xtask::lint_wgsl::{check_fragment, Rule};

const FLOAT_RULES: [Rule; 4] = [
    Rule::IsInfNan,
    Rule::InfNanConstant,
    Rule::SelfCompare,
    Rule::FiniteMax,
];

/// `cmp` (a comparison of `d` against an operand) in a fragment-stage function of `d: ty`, behind the f16 and 16-bit
/// integer enables.
fn src(ty: &str, cmp: &str) -> String {
    format!("enable f16;\nenable wgpu_int16;\nfn g(d: {ty}) -> bool {{ return {cmp}; }}\n")
}

/// Panics, naming every case in `cases` on which `rule` did not fire.
fn all_fire(cases: &[(&str, String)], rule: Rule) {
    let mut missed = Vec::new();
    for (name, s) in cases {
        let found = check_fragment(s)
            .unwrap_or_else(|e| panic!("case `{name}` does not parse/validate: {e}\n{s}"));
        if !found.iter().any(|f| f.rule == rule) {
            missed.push(*name);
        }
    }
    assert!(
        missed.is_empty(),
        "rule {rule} did not fire on {} case(s): {missed:?}",
        missed.len()
    );
}

/// Panics, naming every case in `cases` with a float-rule finding.
fn all_quiet(cases: &[(&str, String)]) {
    let mut loud = Vec::new();
    for (name, s) in cases {
        let found = check_fragment(s)
            .unwrap_or_else(|e| panic!("case `{name}` does not parse/validate: {e}\n{s}"));
        if found.iter().any(|f| FLOAT_RULES.contains(&f.rule)) {
            loud.push(*name);
        }
    }
    assert!(
        loud.is_empty(),
        "float rule finding(s) on near miss(es): {loud:?}"
    );
}

/// Comparisons against inf or NaN computed in 16- and 64-bit integers.
fn inf_nan() -> Vec<(&'static str, String)> {
    vec![
        // 0xffff + 0x7c01 wraps at 16 bits to 0x7c00, f16 inf.
        ("u16 add wraps to f16 inf", src("f16", "d != bitcast<f16>(bitcast<u16>(u16(0xffffu)) + u16(0x7c01u))")),
        // 0xf800 as i16 is negative; >> 1 copies the sign: 0xfc00, f16 -inf.
        ("i16 arithmetic shift to f16 -inf", src("f16", "d != bitcast<f16>(bitcast<i16>(u16(0xf800u)) >> 1u)")),
        // 0x9f00 << 2 = 0x27c00, truncated to 16 bits: 0x7c00, f16 inf.
        ("u16 shift left truncates to f16 inf", src("f16", "d != bitcast<f16>(bitcast<u16>(u16(0x9f00u)) << 2u)")),
        // 0x8400 as i16 is -31744; abs is 31744 = 0x7c00, f16 inf.
        ("i16 abs to f16 inf", src("f16", "d != bitcast<f16>(abs(bitcast<i16>(u16(0x8400u))))")),
        // u16 division by zero gives the dividend: 0x7c00, f16 inf.
        ("u16 divide by zero to f16 inf", src("f16", "d != bitcast<f16>(bitcast<u16>(u16(0x7c00u)) / bitcast<u16>(u16(0u)))")),
        // 0x8000 > 1 unsigned: true, so the select is inf.
        ("u16 unsigned compare selects inf", src("f32", "d != select(1.0f, bitcast<f32>(0x7f800000u), bitcast<u16>(u16(0x8000u)) > u16(1u))")),
        // 0xfff0000000000000 >> 1 unsigned: 0x7ff8000000000000, an f64 NaN.
        ("u64 shift to f64 NaN", src("f64", "d != bitcast<f64>(bitcast<u64>(0xfff0000000000000lu) >> 1u)")),
        // 0xffe0000000000000 as i64 is negative; >> 1 copies the sign: 0xfff0000000000000, f64 -inf.
        ("i64 arithmetic shift to f64 -inf", src("f64", "d != bitcast<f64>(bitcast<i64>(0xffe0000000000000lu) >> 1u)")),
        // Signed max of -inf's bits (negative) and inf's bits (positive) is inf's.
        ("i64 signed max to f64 inf", src("f64", "d != bitcast<f64>(max(bitcast<i64>(0xfff0000000000000lu), bitcast<i64>(0x7ff0000000000000lu)))")),
        // u32 of a u64 keeps the low 32 bits: 0x7f800000, f32 inf.
        ("u64 to u32 truncates to f32 inf", src("f32", "d != bitcast<f32>(u32(bitcast<u64>(0x123456787f800000lu)))")),
        // i32 of an i16 sign-extends: 0xffffff80; & 0x7fffffff is 0x7fffff80, an f32 NaN.
        ("i16 to i32 sign-extends to f32 NaN", src("f32", "d != bitcast<f32>(i32(bitcast<i16>(u16(0xff80u))) & 0x7fffffffi)")),
        // Bits 32..63 of 0x7f80000000000000 are 0x7f800000, f32 inf.
        ("u64 extractBits to f32 inf", src("f32", "d != bitcast<f32>(u32(extractBits(bitcast<u64>(0x7f80000000000000lu), 32u, 32u)))")),
    ]
}

/// Comparisons against a finite-max stand-in computed in 16- and 64-bit integers.
fn finite_max() -> Vec<(&'static str, String)> {
    vec![
        // 0x7c00 - 1 = 0x7bff, f16 65504.
        (
            "u16 inf bits less one are f16 max",
            src(
                "f16",
                "d < bitcast<f16>(bitcast<u16>(u16(0x7c00u)) - u16(1u))",
            ),
        ),
        // 0x47f0000000000000 - 0x20000000 = 0x47efffffe0000000, f32 max in f64.
        (
            "u64 subtract to f32 max in f64",
            src(
                "f64",
                "d < bitcast<f64>(bitcast<u64>(0x47f0000000000000lu) - 0x20000000lu)",
            ),
        ),
        // 0xfbff as i16 is negative: i32 sign-extends to 0xfffffbff; & 0xffff is 0xfbff, f16 -65504, converted to f32.
        (
            "i16 to i32 to f16 -max",
            src(
                "f32",
                "d > f32(bitcast<f16>(u16(u32(i32(bitcast<i16>(u16(0xfbffu)))) & 0xffffu)))",
            ),
        ),
    ]
}

/// Near misses, each one step from a case above: finite, and no stand-in.
fn near_misses() -> Vec<(&'static str, String)> {
    vec![
        // 0xffff + 0x3c01 wraps to 0x3c00, f16 1.0.
        ("u16 add wraps to f16 1.0", src("f16", "d != bitcast<f16>(bitcast<u16>(u16(0xffffu)) + u16(0x3c01u))")),
        // 0xf000 as i16 >> 1: 0xf800, f16 -32768.
        ("i16 arithmetic shift to f16 -32768", src("f16", "d != bitcast<f16>(bitcast<i16>(u16(0xf000u)) >> 1u)")),
        // 0xc400 as i16 is -15360; abs is 15360 = 0x3c00, f16 1.0.
        ("i16 abs to f16 1.0", src("f16", "d != bitcast<f16>(abs(bitcast<i16>(u16(0xc400u))))")),
        // 0x8000 > 1 signed: false, so the select is 1.0.
        ("i16 signed compare selects 1.0", src("f32", "d != select(1.0f, bitcast<f32>(0x7f800000u), bitcast<i16>(u16(0x8000u)) > i16(1))")),
        // 0xfff0000000000000 >> 2 unsigned: 0x3ffc000000000000, f64 1.75.
        ("u64 shift to f64 1.75", src("f64", "d != bitcast<f64>(bitcast<u64>(0xfff0000000000000lu) >> 2u)")),
        // 0xffc0000000000000 as i64 >> 1: 0xffe0000000000000, -f64::MAX, no stand-in.
        ("i64 arithmetic shift to -f64 max", src("f64", "d != bitcast<f64>(bitcast<i64>(0xffc0000000000000lu) >> 1u)")),
        // Signed max of -inf's bits and 1.0's bits is 1.0's (unsigned, it would be -inf's).
        ("i64 signed max to f64 1.0", src("f64", "d != bitcast<f64>(max(bitcast<i64>(0xfff0000000000000lu), bitcast<i64>(0x3ff0000000000000lu)))")),
        // u32 of a u64 keeps the low 32 bits: 0x3f800000, f32 1.0.
        ("u64 to u32 truncates to f32 1.0", src("f32", "d != bitcast<f32>(u32(bitcast<u64>(0x7f8000003f800000lu)))")),
        // u32 of a u16 zero-extends: 0x0000ff80, an f32 subnormal (as i16, it would sign-extend to a NaN).
        ("u16 to u32 zero-extends to a subnormal", src("f32", "d != bitcast<f32>(u32(bitcast<u16>(u16(0xff80u))) & 0x7fffffffu)")),
        // Bits 31..62 of 0x7f80000000000000 are 0xff000000, f32 -2^127.
        ("u64 extractBits to f32 -2^127", src("f32", "d != bitcast<f32>(u32(extractBits(bitcast<u64>(0x7f80000000000000lu), 31u, 32u)))")),
        // 0x7c00 - 2 = 0x7bfe, f16 65472.
        ("u16 inf bits less two are f16 65472", src("f16", "d < bitcast<f16>(bitcast<u16>(u16(0x7c00u)) - u16(2u))")),
        // 0x47f0000000000000 - 0x20000001 is one ulp below f32 max in f64.
        ("u64 subtract to just below f32 max in f64", src("f64", "d < bitcast<f64>(bitcast<u64>(0x47f0000000000000lu) - 0x20000001lu)")),
        // 0xfbfe as i16, through i32 and masked: 0xfbfe, f16 -65472.
        ("i16 to i32 to f16 -65472", src("f32", "d > f32(bitcast<f16>(u16(u32(i32(bitcast<i16>(u16(0xfbfeu)))) & 0xffffu)))")),
    ]
}

#[test]
fn qa_lint_wgsl_wide_inf_nan_fires() {
    all_fire(&inf_nan(), Rule::InfNanConstant);
}

negative_control!(
    qa_lint_wgsl_wide_inf_nan_fires,
    "the near misses evaluate to no inf or NaN, so the rule must not fire",
    expected = "did not fire",
    all_fire(&near_misses(), Rule::InfNanConstant)
);

#[test]
fn qa_lint_wgsl_wide_finite_max_fires() {
    all_fire(&finite_max(), Rule::FiniteMax);
}

negative_control!(
    qa_lint_wgsl_wide_finite_max_fires,
    "the near misses evaluate to no stand-in, so the rule must not fire",
    expected = "did not fire",
    all_fire(&near_misses(), Rule::FiniteMax)
);

#[test]
fn qa_lint_wgsl_wide_near_misses_are_quiet() {
    all_quiet(&near_misses());
}

negative_control!(
    qa_lint_wgsl_wide_near_misses_are_quiet,
    "the inf and NaN cases have findings, so the quiet check must fail",
    expected = "near miss",
    all_quiet(&inf_nan())
);
