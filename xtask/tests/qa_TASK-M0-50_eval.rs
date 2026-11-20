//! QA tests for TASK-M0-50, round 2, written from REQ-RENDER-083: a comparison against "an inf or NaN constant (a
//! constant expression that evaluates to one included)" and against a finite-max stand-in "as a `bitcast<f32>` of its
//! bit pattern or as another constant expression that evaluates to it" must fail the lint (R-351, R-352, R-353).
//!
//! WGSL evaluates a constant expression in its operands' type: f32 arithmetic rounds to f32 (so the largest f32 plus
//! one is the largest f32, and twice it overflows to inf), and f16 arithmetic rounds to f16, ties to even (65504 + 8
//! rounds back to 65504; 65504 + 16, the midpoint to 2^16, overflows to inf). The cases below are worked out from
//! that, independently of the lint, and cover WGSL's constant built-ins (`ldexp`, `select`, `fma`, `mix`, `step`,
//! `fract`, `dot`, `length`) and an integer-to-float conversion, as well as arithmetic, components and rounding
//! built-ins. The near misses, which evaluate to a finite value other than a stand-in, must stay quiet. Each test has a
//! registered negative control (R-176).

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

// ---------------------------------------------------------------------------------------------------------------
// Arithmetic, components and rounding built-ins over a bitcast.

const INF_NAN_ARITH: [(&str, &str); 12] = [
    ("f32 max * 2 overflows to inf", "fn g(d: f32) -> bool { return d == bitcast<f32>(0x7f7fffffu) * 2.0; }"),
    ("f32 max + 1e32 overflows to inf in f32", "fn g(d: f32) -> bool { return d == bitcast<f32>(0x7f7fffffu) + 1.0e32; }"),
    ("inf - inf is NaN", "fn g(d: f32) -> bool { return d != bitcast<f32>(0x7f800000u) - bitcast<f32>(0x7f800000u); }"),
    ("0 / 0 is NaN", "fn g(d: f32) -> bool { return d != bitcast<f32>(0u) / bitcast<f32>(0u); }"),
    ("1 / 0 is inf", "fn g(d: f32) -> bool { return d < 1.0 / bitcast<f32>(0u); }"),
    ("max(inf, 1)", "fn g(d: f32) -> bool { return d < max(bitcast<f32>(0x7f800000u), 1.0); }"),
    ("a vector's component by index", "fn g(d: f32) -> bool { return d < vec2<f32>(1.0, bitcast<f32>(0x7f800000u))[1]; }"),
    ("a swizzle", "fn g(d: vec2<f32>) -> vec2<bool> { return d < vec2<f32>(1.0, bitcast<f32>(0x7f800000u)).yx; }"),
    ("a matrix column", "fn g(d: vec2<f32>) -> vec2<bool> { return d < mat2x2<f32>(vec2<f32>(1.0), vec2<f32>(bitcast<f32>(0x7f800000u)))[1]; }"),
    ("an array element", "fn g(d: f32) -> bool { return d < array<f32, 2>(1.0, bitcast<f32>(0x7f800000u))[1]; }"),
    ("f16 65504 + 16 rounds to inf", "enable f16;\nfn g(h: f16) -> bool { return h < f16(bitcast<f32>(0x477fe000u)) + 16.0h; }"),
    ("a vector product overflowing", "fn g(d: vec2<f32>) -> vec2<bool> { return d < vec2<f32>(bitcast<f32>(0x7f7fffffu)) * vec2<f32>(2.0, 2.0); }"),
];

const FINITE_MAX_ARITH: [(&str, &str); 11] = [
    ("f32 max + 1 rounds to f32 max", "fn g(d: f32) -> bool { return d < bitcast<f32>(0x7f7fffffu) + 1.0; }"),
    ("min(f32 max, inf)", "fn g(d: f32) -> bool { return d < min(bitcast<f32>(0x7f7fffffu), bitcast<f32>(0x7f800000u)); }"),
    ("clamp(inf, 0, 65504)", "fn g(d: f32) -> bool { return d < clamp(bitcast<f32>(0x7f800000u), 0.0, 65504.0); }"),
    ("floor(65504.5)", "fn g(d: f32) -> bool { return d < floor(bitcast<f32>(0x477fe000u) + 0.5); }"),
    ("ceil(65503.5)", "fn g(d: f32) -> bool { return d < ceil(bitcast<f32>(0x477fdf80u)); }"),
    ("round(65503.5) ties to even", "fn g(d: f32) -> bool { return d < round(bitcast<f32>(0x477fe000u) - 0.5); }"),
    ("trunc(-65504.5)", "fn g(d: f32) -> bool { return d < trunc(-bitcast<f32>(0x477fe000u) - 0.5); }"),
    ("sign(inf) * 65504", "fn g(d: f32) -> bool { return d < sign(bitcast<f32>(0x7f800000u)) * 65504.0; }"),
    ("65505 % 65505 + 65504", "fn g(d: f32) -> bool { return d < (bitcast<f32>(0x477fe000u) + 1.0) % 65505.0 + 65504.0; }"),
    ("f16 65504 + 8 rounds to 65504", "enable f16;\nfn g(h: f16) -> bool { return h < f16(bitcast<f32>(0x477fe000u)) + 8.0h; }"),
    ("f16(65519.0) rounds to 65504", "enable f16;\nfn g(h: f16) -> bool { return h < f16(bitcast<f32>(0x477fef00u)); }"),
];

/// Finite values next to the stand-ins, and components and branches that are not the inf.
const ARITH_QUIET: [(&str, &str); 9] = [
    ("f32 max's predecessor + 1 stays below f32 max", "fn g(d: f32) -> bool { return d < bitcast<f32>(0x7f7ffffeu) + 1.0; }"),
    ("65504 + 1 is 65505 in f32", "fn g(d: f32) -> bool { return d < bitcast<f32>(0x477fe000u) + 1.0; }"),
    ("65504 - 0.5", "fn g(d: f32) -> bool { return d < bitcast<f32>(0x477fe000u) - 0.5; }"),
    ("f16 65504 - 16 is 65488", "enable f16;\nfn g(h: f16) -> bool { return h < f16(bitcast<f32>(0x477fe000u)) - 16.0h; }"),
    ("round(65502.5) ties to even, 65502", "fn g(d: f32) -> bool { return d < round(bitcast<f32>(0x477fe000u) - 1.5); }"),
    ("the finite component of a vector holding inf", "fn g(d: f32) -> bool { return d < vec2<f32>(1.0, bitcast<f32>(0x7f800000u)).x; }"),
    ("the finite element of an array holding inf", "fn g(d: f32) -> bool { return d < array<f32, 2>(1.0, bitcast<f32>(0x7f800000u))[0]; }"),
    ("f32 max times a runtime value", "fn g(d: f32, e: f32) -> bool { return d < bitcast<f32>(0x7f7fffffu) * e; }"),
    ("(65504 + 1) % 65505 + 65503", "fn g(d: f32) -> bool { return d < (bitcast<f32>(0x477fe000u) + 1.0) % 65505.0 + 65503.0; }"),
];

#[test]
fn qa_lint_wgsl_eval_arith_inf_nan_fires() {
    all_fire(&INF_NAN_ARITH, Rule::InfNanConstant);
}

negative_control!(
    qa_lint_wgsl_eval_arith_inf_nan_fires,
    "finite constant expressions are no inf/NaN constant",
    expected = "did not fire",
    all_fire(&ARITH_QUIET, Rule::InfNanConstant)
);

#[test]
fn qa_lint_wgsl_eval_arith_finite_max_fires() {
    all_fire(&FINITE_MAX_ARITH, Rule::FiniteMax);
}

negative_control!(
    qa_lint_wgsl_eval_arith_finite_max_fires,
    "neighbours of the stand-ins are no finite-max comparison",
    expected = "did not fire",
    all_fire(&ARITH_QUIET, Rule::FiniteMax)
);

#[test]
fn qa_lint_wgsl_eval_arith_near_misses_are_quiet() {
    all_quiet(&ARITH_QUIET);
}

negative_control!(
    qa_lint_wgsl_eval_arith_near_misses_are_quiet,
    "constant expressions evaluating to inf, NaN or a stand-in are float-rule findings",
    expected = "float rule finding(s)",
    all_quiet(&FINITE_MAX_ARITH)
);

// ---------------------------------------------------------------------------------------------------------------
// WGSL's other constant built-ins, and an integer-to-float conversion naga leaves unfolded.

const INF_NAN_BUILTINS: [(&str, &str); 3] = [
    ("ldexp(1.0, 128), literals only", "fn g(d: f32) -> bool { return d < ldexp(1.0, 128); }"),
    ("dot of an inf vector", "fn g(d: f32) -> bool { return d < dot(vec2<f32>(bitcast<f32>(0x7f800000u)), vec2<f32>(1.0)); }"),
    ("length of an inf vector", "fn g(d: f32) -> bool { return d < length(vec2<f32>(bitcast<f32>(0x7f800000u))); }"),
];

const FINITE_MAX_BUILTINS: [(&str, &str); 8] = [
    (
        "ldexp(2047.0, 5), literals only",
        "fn g(d: f32) -> bool { return d < ldexp(2047.0, 5); }",
    ),
    (
        "ldexp over a bitcast",
        "fn g(d: f32) -> bool { return d < ldexp(bitcast<f32>(0x44ffe000u), 5); }",
    ),
    (
        "select(1.0, 65504 by bits, true)",
        "fn g(d: f32) -> bool { return d < select(1.0, bitcast<f32>(0x477fe000u), true); }",
    ),
    (
        "fma(32752 by bits, 2, 0)",
        "fn g(d: f32) -> bool { return d < fma(bitcast<f32>(0x46ffe000u), 2.0, 0.0); }",
    ),
    (
        "mix(0, 65504 by bits, 1)",
        "fn g(d: f32) -> bool { return d < mix(0.0, bitcast<f32>(0x477fe000u), 1.0); }",
    ),
    (
        "step(0, 1 by bits) * 65504",
        "fn g(d: f32) -> bool { return d < step(0.0, bitcast<f32>(0x3f800000u)) * 65504.0; }",
    ),
    (
        "fract(1 by bits) + 65504",
        "fn g(d: f32) -> bool { return d < fract(bitcast<f32>(0x3f800000u)) + 65504.0; }",
    ),
    (
        "f32(bitcast<u32>(65504i))",
        "fn g(d: f32) -> bool { return d < f32(bitcast<u32>(65504i)); }",
    ),
];

/// The same built-ins evaluating to finite values other than a stand-in.
const BUILTINS_QUIET: [(&str, &str); 7] = [
    ("ldexp(2047.0, 4) is 32752", "fn g(d: f32) -> bool { return d < ldexp(2047.0, 4); }"),
    ("ldexp(1.0, 127) is 2^127", "fn g(d: f32) -> bool { return d < ldexp(1.0, 127); }"),
    ("select(1.0, 65504 by bits, false)", "fn g(d: f32) -> bool { return d < select(1.0, bitcast<f32>(0x477fe000u), false); }"),
    ("fma(32752 by bits, 2, 1) is 65505", "fn g(d: f32) -> bool { return d < fma(bitcast<f32>(0x46ffe000u), 2.0, 1.0); }"),
    ("mix(0, 65504 by bits, 0.5)", "fn g(d: f32) -> bool { return d < mix(0.0, bitcast<f32>(0x477fe000u), 0.5); }"),
    ("dot of a finite vector", "fn g(d: f32) -> bool { return d < dot(vec2<f32>(bitcast<f32>(0x3f800000u)), vec2<f32>(1.0)); }"),
    ("f32(bitcast<u32>(65503i))", "fn g(d: f32) -> bool { return d < f32(bitcast<u32>(65503i)); }"),
];

#[test]
fn qa_lint_wgsl_eval_builtins_inf_nan_fires() {
    all_fire(&INF_NAN_BUILTINS, Rule::InfNanConstant);
}

negative_control!(
    qa_lint_wgsl_eval_builtins_inf_nan_fires,
    "built-ins evaluating to finite values are no inf/NaN constant",
    expected = "did not fire",
    all_fire(&BUILTINS_QUIET, Rule::InfNanConstant)
);

#[test]
fn qa_lint_wgsl_eval_builtins_finite_max_fires() {
    all_fire(&FINITE_MAX_BUILTINS, Rule::FiniteMax);
}

negative_control!(
    qa_lint_wgsl_eval_builtins_finite_max_fires,
    "built-ins evaluating to other finite values are no finite-max comparison",
    expected = "did not fire",
    all_fire(&BUILTINS_QUIET, Rule::FiniteMax)
);

#[test]
fn qa_lint_wgsl_eval_builtins_near_misses_are_quiet() {
    all_quiet(&BUILTINS_QUIET);
}

negative_control!(
    qa_lint_wgsl_eval_builtins_near_misses_are_quiet,
    "comparisons against a constant stand-in are float-rule findings",
    expected = "float rule finding(s)",
    all_quiet(&[("65504.0", "fn g(d: f32) -> bool { return d < 65504.0; }")])
);
