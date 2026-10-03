//! QA tests for TASK-M0-50, round 5, written from REQ-RENDER-083: every float rule (a use of `isinf` or `isnan`, a
//! comparison against an inf or NaN constant, against a finite-max stand-in, or of a float with itself) must fail the
//! lint wherever the use sits in a function, a bare `{ … }` block statement (nested or not) included; and "no store
//! between" the two reads of a self-comparison must see a store made inside a bare block.
//!
//! Each firing case has a near miss that must stay quiet, and each test has a registered negative control (R-176).

use validation::negative_control;
use xtask::lint_wgsl::{check_fragment, Rule};

const FLOAT_RULES: [Rule; 4] = [
    Rule::IsInfNan,
    Rule::InfNanConstant,
    Rule::SelfCompare,
    Rule::FiniteMax,
];

/// `body` as the body of a fragment-stage function of `x: f32` and `c: bool`, beside hand-written `isinf` and `isnan`.
fn src(body: &str) -> String {
    format!(
        "fn isinf(x: f32) -> bool {{ return (bitcast<u32>(x) & 0x7fffffffu) == 0x7f800000u; }}\n\
         fn isnan(x: f32) -> bool {{ return (bitcast<u32>(x) & 0x7fffffffu) > 0x7f800000u; }}\n\
         fn isfin(x: f32) -> bool {{ return (bitcast<u32>(x) & 0x7f800000u) != 0x7f800000u; }}\n\
         fn f(x: f32, c: bool) -> bool {{\n{body}\n}}\n"
    )
}

/// Each `(rule, body)`: a use `rule` names, inside one or more bare blocks.
const IN_BLOCKS: [(Rule, &str); 8] = [
    (Rule::IsInfNan, "{ return isnan(x); }"),
    (
        Rule::IsInfNan,
        "{ { if c { return isinf(x); } } } return false;",
    ),
    (
        Rule::InfNanConstant,
        "{ return x == bitcast<f32>(0x7f800000u); }",
    ),
    (
        Rule::InfNanConstant,
        "{ { return x != bitcast<f32>(0x7fc00000u); } }",
    ),
    (Rule::FiniteMax, "{ return x >= 65504.0; }"),
    (Rule::FiniteMax, "{ { return -3.40282347e38 > x; } }"),
    (Rule::SelfCompare, "var v = x; { { return v != v; } }"),
    (
        Rule::SelfCompare,
        "var v = x; var u = x; let old = v; { u = 1.0; } { } return old != v;",
    ),
];

/// Each a near miss of the `IN_BLOCKS` case at the same index: a call to another function, a finite value that is no
/// stand-in, or a store to the place read made inside a bare block between the two reads.
const IN_BLOCKS_NEAR: [&str; 8] = [
    "{ return isfin(x); }",
    "{ { if c { return isfin(x); } } } return false;",
    "{ return x == bitcast<f32>(0x7f7ffffeu); }",
    "{ { return x != bitcast<f32>(0x3fc00000u); } }",
    "{ return x >= 65503.0; }",
    "{ { return -3.40282e38 > x; } }",
    "var v = x; let old = v; { v = 1.0; } return old != v;",
    "var v = x; var u = x; let old = v; { u = 1.0; { v = v + 1.0; } } return old != v;",
];

/// Panics, naming every case on which its rule did not fire.
fn all_fire(cases: &[(Rule, &str)]) {
    let mut missed = Vec::new();
    for &(rule, body) in cases {
        let s = src(body);
        let found = check_fragment(&s)
            .unwrap_or_else(|e| panic!("case `{body}` does not parse/validate: {e}\n{s}"));
        if !found.iter().any(|f| f.rule == rule) {
            missed.push((rule, body));
        }
    }
    assert!(missed.is_empty(), "did not fire on: {missed:?}");
}

/// Panics, naming every case with a float-rule finding.
fn all_quiet(cases: &[&str]) {
    let mut loud = Vec::new();
    for body in cases {
        let s = src(body);
        let found = check_fragment(&s)
            .unwrap_or_else(|e| panic!("case `{body}` does not parse/validate: {e}\n{s}"));
        if found.iter().any(|f| FLOAT_RULES.contains(&f.rule)) {
            loud.push((*body, found));
        }
    }
    assert!(loud.is_empty(), "a near miss fired: {loud:?}");
}

#[test]
fn qa_m0_50_every_float_rule_fires_inside_bare_blocks() {
    all_fire(&IN_BLOCKS);
}

negative_control!(
    qa_m0_50_every_float_rule_fires_inside_bare_blocks,
    "the near misses carry no use any float rule names",
    expected = "did not fire",
    all_fire(
        &IN_BLOCKS
            .iter()
            .zip(IN_BLOCKS_NEAR)
            .map(|(&(rule, _), near)| (rule, near))
            .collect::<Vec<_>>()
    )
);

#[test]
fn qa_m0_50_near_misses_inside_bare_blocks_stay_quiet() {
    all_quiet(&IN_BLOCKS_NEAR);
}

negative_control!(
    qa_m0_50_near_misses_inside_bare_blocks_stay_quiet,
    "the firing cases are findings",
    expected = "a near miss fired",
    all_quiet(&IN_BLOCKS.iter().map(|&(_, b)| b).collect::<Vec<_>>())
);
