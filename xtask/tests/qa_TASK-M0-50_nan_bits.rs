//! QA tests for TASK-M0-50, round 3, written from REQ-RENDER-083: a comparison against "an inf or NaN constant (a
//! constant expression that evaluates to one included)" and against a finite-max stand-in "as a `bitcast<f32>` of its
//! bit pattern or as another constant expression that evaluates to it" must fail the lint (R-351, R-352, R-353).
//!
//! A NaN's bits are as constant as any other float's: `bitcast<u32>` of `bitcast<f32>(0x7fc00000u)` is `0x7fc00000u`
//! exactly, and so a `bitcast<f32>` back is that NaN, its high bits masked with `0x7f800000u` are inf, and
//! `0x7fc00000u - 0x00400001u` is `0x7f7fffffu`, f32's largest finite value. Each value below is worked out from the
//! bit patterns by hand, independently of the lint. The near misses evaluate to finite values other than a stand-in
//! and must stay quiet. Each test has a registered negative control (R-176).

use validation::negative_control;
use xtask::lint_wgsl::{check_fragment, Rule};

const FLOAT_RULES: [Rule; 4] = [
    Rule::IsInfNan,
    Rule::InfNanConstant,
    Rule::SelfCompare,
    Rule::FiniteMax,
];

/// Panics, naming every case in `cases` on which `rule` did not fire.
fn all_fire(cases: &[(&str, &str)], rule: Rule) {
    let mut missed = Vec::new();
    for (name, src) in cases {
        let found = check_fragment(src)
            .unwrap_or_else(|e| panic!("case `{name}` does not parse/validate: {e}\n{src}"));
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
fn all_quiet(cases: &[(&str, &str)]) {
    let mut loud = Vec::new();
    for (name, src) in cases {
        let found = check_fragment(src)
            .unwrap_or_else(|e| panic!("case `{name}` does not parse/validate: {e}\n{src}"));
        if found.iter().any(|f| FLOAT_RULES.contains(&f.rule)) {
            loud.push(*name);
        }
    }
    assert!(
        loud.is_empty(),
        "float rule finding(s) on near miss(es): {loud:?}"
    );
}

/// Constant expressions over a NaN's bits that evaluate to NaN or inf.
const NAN_BITS_INF_NAN: [(&str, &str); 4] = [
    (
        "a quiet NaN through u32 and back is NaN",
        "fn g(d: f32) -> bool { return d != bitcast<f32>(bitcast<u32>(bitcast<f32>(0x7fc00000u))); }",
    ),
    (
        "a negative NaN with a payload through i32 and back is NaN",
        "fn g(d: f32) -> bool { return d != bitcast<f32>(bitcast<i32>(bitcast<f32>(0xffc00001u))); }",
    ),
    (
        "a NaN's bits masked to the exponent are inf",
        "fn g(d: f32) -> bool { return d < bitcast<f32>(bitcast<u32>(bitcast<f32>(0x7fc00000u)) & 0x7f800000u); }",
    ),
    (
        "a vector lane holding a NaN through u32 and back",
        "fn g(d: vec2<f32>) -> vec2<bool> { return d != bitcast<vec2<f32>>(bitcast<vec2<u32>>(vec2<f32>(bitcast<f32>(0x7fc00000u), 1.0))); }",
    ),
];

/// A constant expression over a NaN's bits that evaluates to f32's largest finite value.
const NAN_BITS_FINITE_MAX: [(&str, &str); 1] = [(
    "a NaN's bits less 0x00400001 are f32 max",
    "fn g(d: f32) -> bool { return d < bitcast<f32>(bitcast<u32>(bitcast<f32>(0x7fc00000u)) - 0x00400001u); }",
)];

/// Constant expressions over a NaN's bits, or a float's round trip through its bits, that evaluate to a finite value
/// other than a stand-in.
const NAN_BITS_QUIET: [(&str, &str); 4] = [
    (
        "a NaN's bits masked with 1.0's are 1.0",
        "fn g(d: f32) -> bool { return d < bitcast<f32>(bitcast<u32>(bitcast<f32>(0x7fc00000u)) & 0x3f800000u); }",
    ),
    (
        "a NaN's bits less 0x00400002 are just below f32 max",
        "fn g(d: f32) -> bool { return d < bitcast<f32>(bitcast<u32>(bitcast<f32>(0x7fc00000u)) - 0x00400002u); }",
    ),
    (
        "1.0 through u32 and back",
        "fn g(d: f32) -> bool { return d < bitcast<f32>(bitcast<u32>(bitcast<f32>(0x3f800000u))); }",
    ),
    (
        "the finite lane of a vector holding a NaN, through u32 and back",
        "fn g(d: f32) -> bool { return d < bitcast<vec2<f32>>(bitcast<vec2<u32>>(vec2<f32>(bitcast<f32>(0x7fc00000u), 1.0))).y; }",
    ),
];

#[test]
fn qa_lint_wgsl_nan_bits_inf_nan_fires() {
    all_fire(&NAN_BITS_INF_NAN, Rule::InfNanConstant);
}

negative_control!(
    qa_lint_wgsl_nan_bits_inf_nan_fires,
    "constant expressions over a NaN's bits evaluating to finite values are no inf/NaN constant",
    expected = "did not fire",
    all_fire(&NAN_BITS_QUIET, Rule::InfNanConstant)
);

#[test]
fn qa_lint_wgsl_nan_bits_finite_max_fires() {
    all_fire(&NAN_BITS_FINITE_MAX, Rule::FiniteMax);
}

negative_control!(
    qa_lint_wgsl_nan_bits_finite_max_fires,
    "constant expressions over a NaN's bits evaluating to other finite values are no finite-max comparison",
    expected = "did not fire",
    all_fire(&NAN_BITS_QUIET, Rule::FiniteMax)
);

#[test]
fn qa_lint_wgsl_nan_bits_near_misses_are_quiet() {
    all_quiet(&NAN_BITS_QUIET);
}

negative_control!(
    qa_lint_wgsl_nan_bits_near_misses_are_quiet,
    "a comparison against a NaN by its bits is a float-rule finding",
    expected = "float rule finding(s)",
    all_quiet(&[(
        "a NaN by its bits",
        "fn g(d: f32) -> bool { return d != bitcast<f32>(0x7fc00000u); }"
    )])
);
