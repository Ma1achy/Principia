//! The float rules over a NaN's bit pattern and over 16- and 64-bit integers (REQ-RENDER-083; R-351, R-352): a
//! comparison against "an inf or NaN constant (a constant expression that evaluates to one included)" or against a
//! finite-max stand-in "as a `bitcast<f32>` of its bit pattern or as another constant expression that evaluates to it"
//! fails the lint. A NaN a `bitcast` made has bits as known as any other value's: its sign and payload, for f32, for
//! f16 (through `u16` and `i16`, under `enable wgpu_int16`) and for f64 (through `u64`). Each near miss evaluates to a
//! finite value other than a stand-in and stays quiet. Each test has a registered negative control (R-176).

use validation::negative_control;
use xtask::lint_wgsl::{check_fragment, Rule};

const FLOAT_RULES: [Rule; 4] = [
    Rule::IsInfNan,
    Rule::InfNanConstant,
    Rule::SelfCompare,
    Rule::FiniteMax,
];

/// The f16 header: f16 and 16-bit integers enabled.
const H: &str = "enable f16;\nenable wgpu_int16;\n";

/// `body` as a module, behind the f16 header.
fn module(body: &str) -> String {
    format!("{H}{body}")
}

/// Panics, naming every case in `cases` on which `rule` did not fire.
fn all_fire(cases: &[(&str, String)], rule: Rule) {
    let missed: Vec<&str> = cases
        .iter()
        .filter(|(name, src)| {
            let found = check_fragment(src)
                .unwrap_or_else(|e| panic!("case `{name}` does not parse/validate: {e}\n{src}"));
            !found.iter().any(|f| f.rule == rule)
        })
        .map(|(name, _)| *name)
        .collect();
    assert!(missed.is_empty(), "rule {rule} did not fire on: {missed:?}");
}

/// Panics, naming every case in `cases` with a float-rule finding.
fn all_quiet(cases: &[(&str, String)]) {
    let loud: Vec<&str> = cases
        .iter()
        .filter(|(name, src)| {
            let found = check_fragment(src)
                .unwrap_or_else(|e| panic!("case `{name}` does not parse/validate: {e}\n{src}"));
            found.iter().any(|f| FLOAT_RULES.contains(&f.rule))
        })
        .map(|(name, _)| *name)
        .collect();
    assert!(
        loud.is_empty(),
        "float rule finding(s) on near miss(es): {loud:?}"
    );
}

/// Comparisons against a NaN or inf built from a NaN's bits, or a 16- or 64-bit integer's.
fn inf_nan() -> Vec<(&'static str, String)> {
    vec![
        (
            "a negative NaN's bits shifted right are NaN (its sign kept)",
            module("fn g(d: f32) -> bool { return d != bitcast<f32>(bitcast<u32>(bitcast<f32>(0xffc00000u)) >> 1u); }"),
        ),
        (
            "a NaN's bits through a let, masked to its exponent, are inf",
            module("fn g(d: f32) -> bool { let n = bitcast<f32>(0x7fc00000u); return d != bitcast<f32>(bitcast<u32>(n) & 0x7f800000u); }"),
        ),
        (
            "a vector's NaN lane masked to its exponent is inf",
            module("fn g(d: f32) -> bool { return d != bitcast<vec2<f32>>(bitcast<vec2<u32>>(vec2<f32>(1.0, bitcast<f32>(0xffc00000u))) & vec2<u32>(0x7f800000u)).y; }"),
        ),
        (
            "an f16 NaN's bits through u16, masked to its exponent, are inf",
            module("fn g(d: f16) -> bool { return d != bitcast<f16>(bitcast<u16>(bitcast<f16>(u16(0xfe01u))) & u16(0x7c00u)); }"),
        ),
        (
            "an f16 NaN through i16 and back is NaN",
            module("fn g(d: f16) -> bool { return d != bitcast<f16>(bitcast<i16>(bitcast<f16>(u16(0x7e00u)))); }"),
        ),
        (
            "an f16 inf from a u16",
            module("fn g(d: f16) -> bool { return d != bitcast<f16>(u16(0x7c00u)); }"),
        ),
        (
            "an f64 inf from a u64",
            module("fn g(d: f64) -> bool { return d != bitcast<f64>(0x7ff0000000000000lu); }"),
        ),
        (
            "an f64 NaN's bits masked to its exponent are inf",
            module("fn g(d: f64) -> bool { return d < bitcast<f64>(bitcast<u64>(bitcast<f64>(0xfff8000000000001lu)) & 0x7ff0000000000000lu); }"),
        ),
    ]
}

/// Comparisons against a finite-max stand-in built from a NaN's bits.
fn finite_max() -> Vec<(&'static str, String)> {
    vec![
        (
            "a signalling NaN's payload less 2 is f32 max",
            module("fn g(d: f32) -> bool { return d < bitcast<f32>(bitcast<u32>(bitcast<f32>(0x7f800001u)) - 2u); }"),
        ),
        (
            "an f16 NaN's bits less 0x0201 are 65504",
            module("fn g(d: f16) -> bool { return d < bitcast<f16>(bitcast<u16>(bitcast<f16>(u16(0x7e00u))) - u16(0x0201u)); }"),
        ),
        (
            "an f64 NaN's bits less 0x3808000020000000 are f32 max, in f64",
            module("fn g(d: f64) -> bool { return d < bitcast<f64>(bitcast<u64>(bitcast<f64>(0x7ff8000000000000lu)) - 0x3808000020000000lu); }"),
        ),
    ]
}

/// The near misses: each one bit or one step from a case above, finite and no stand-in.
fn near_misses() -> Vec<(&'static str, String)> {
    vec![
        (
            "a positive NaN's bits shifted right are 1.75",
            module("fn g(d: f32) -> bool { return d != bitcast<f32>(bitcast<u32>(bitcast<f32>(0x7fc00000u)) >> 1u); }"),
        ),
        (
            "a vector's finite lane masked to its exponent is 1.0",
            module("fn g(d: f32) -> bool { return d != bitcast<vec2<f32>>(bitcast<vec2<u32>>(vec2<f32>(1.0, bitcast<f32>(0xffc00000u))) & vec2<u32>(0x7f800000u)).x; }"),
        ),
        (
            "a signalling NaN's payload less 3 is just below f32 max",
            module("fn g(d: f32) -> bool { return d < bitcast<f32>(bitcast<u32>(bitcast<f32>(0x7f800001u)) - 3u); }"),
        ),
        (
            "an f16 NaN's bits less 0x0202 are 65472",
            module("fn g(d: f16) -> bool { return d < bitcast<f16>(bitcast<u16>(bitcast<f16>(u16(0x7e00u))) - u16(0x0202u)); }"),
        ),
        (
            "an f16 NaN's bits masked to its exponent's low bits are 1.0",
            module("fn g(d: f16) -> bool { return d != bitcast<f16>(bitcast<u16>(bitcast<f16>(u16(0xfe01u))) & u16(0x3c00u)); }"),
        ),
        (
            "f64 max, which is no stand-in",
            module("fn g(d: f64) -> bool { return d < bitcast<f64>(0x7feffffffffffffflu); }"),
        ),
        (
            "an f64 NaN's bits less 0x3808000020000001 are just below f32 max, in f64",
            module("fn g(d: f64) -> bool { return d < bitcast<f64>(bitcast<u64>(bitcast<f64>(0x7ff8000000000000lu)) - 0x3808000020000001lu); }"),
        ),
    ]
}

#[test]
fn lint_wgsl_bits_inf_nan_fires() {
    all_fire(&inf_nan(), Rule::InfNanConstant);
}

negative_control!(
    lint_wgsl_bits_inf_nan_fires,
    "the near misses evaluate to no inf or NaN, so the rule must not fire",
    expected = "did not fire",
    all_fire(&near_misses(), Rule::InfNanConstant)
);

#[test]
fn lint_wgsl_bits_finite_max_fires() {
    all_fire(&finite_max(), Rule::FiniteMax);
}

negative_control!(
    lint_wgsl_bits_finite_max_fires,
    "the near misses evaluate to no stand-in, so the rule must not fire",
    expected = "did not fire",
    all_fire(&near_misses(), Rule::FiniteMax)
);

#[test]
fn lint_wgsl_bits_near_misses_are_quiet() {
    all_quiet(&near_misses());
}

negative_control!(
    lint_wgsl_bits_near_misses_are_quiet,
    "the inf and NaN cases have findings, so the quiet check must fail",
    expected = "near miss",
    all_quiet(&inf_nan())
);
