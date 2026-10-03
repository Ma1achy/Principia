//! `cargo xtask lint wgsl` (render contract Part 5, "Unpack layer"; REQ-RENDER-001): it passes on the generated WGSL
//! and on a clean fixture, and each fixture that breaks one rule fails it, naming that rule: an i32 `extractBits`, an
//! f64, `enable f16`, an `r` not vec2-grouped, and a word inside `SimState`; and, for R-343's bindings, the state
//! buffer in group 0, the word buffer at the wrong binding number, and `word_buffer` indexed outside `sample_word`.
//! The generated read side (lowering Part 3a) passes as the generated layer's continuation, its findings reported at
//! its own lines, and its read-side `SimState` may hold the word it read.

use std::path::{Path, PathBuf};

use validation::negative_control;
use xtask::lint_wgsl::{check, check_read_side, Rule, GENERATED, READ_SIDE_FILE};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/lint_wgsl")
        .join(name)
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The lint finds nothing in `path`.
fn check_passes(path: &Path) {
    let found = check(&read(path)).expect("the WGSL parses and validates");
    assert!(
        found.is_empty(),
        "lint wgsl finding(s) in {}: {found:?}",
        path.display()
    );
}

/// The lint fails `path` with findings of `rule` only, at least one, each naming the rule.
fn check_fails_naming(path: &Path, rule: Rule) {
    let found = check(&read(path)).expect("the fixture parses and validates");
    assert!(
        !found.is_empty(),
        "no finding in {}: rule {rule} did not fire",
        path.display()
    );
    for f in &found {
        assert_eq!(f.rule, rule, "a finding of another rule: {f}");
        assert!(
            f.to_string().contains(&format!("[{rule}]")),
            "the finding does not name its rule: {f}"
        );
    }
}

#[test]
fn lint_wgsl_passes_on_the_generated_wgsl() {
    check_passes(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(GENERATED),
    );
}

negative_control!(
    lint_wgsl_passes_on_the_generated_wgsl,
    "a file with an i32 extractBits has a finding, so the passing check must fail",
    expected = "lint wgsl finding(s)",
    check_passes(&fixture("extract_bits_i32.wgsl"))
);

#[test]
fn lint_wgsl_passes_on_the_clean_fixture() {
    check_passes(&fixture("clean.wgsl"));
}

negative_control!(
    lint_wgsl_passes_on_the_clean_fixture,
    "a file with an f64 has a finding, so the passing check must fail",
    expected = "lint wgsl finding(s)",
    check_passes(&fixture("f64.wgsl"))
);

#[test]
fn lint_wgsl_i32_extract_bits_fails_naming_the_rule() {
    check_fails_naming(&fixture("extract_bits_i32.wgsl"), Rule::ExtractBitsU32);
}

negative_control!(
    lint_wgsl_i32_extract_bits_fails_naming_the_rule,
    "the clean fixture has no i32 extractBits, so the rule must not fire",
    expected = "did not fire",
    check_fails_naming(&fixture("clean.wgsl"), Rule::ExtractBitsU32)
);

#[test]
fn lint_wgsl_f64_fails_naming_the_rule() {
    check_fails_naming(&fixture("f64.wgsl"), Rule::NoF64);
}

negative_control!(
    lint_wgsl_f64_fails_naming_the_rule,
    "the clean fixture has no f64, so the rule must not fire",
    expected = "did not fire",
    check_fails_naming(&fixture("clean.wgsl"), Rule::NoF64)
);

#[test]
fn lint_wgsl_enable_f16_fails_naming_the_rule() {
    check_fails_naming(&fixture("enable_f16.wgsl"), Rule::NoEnableF16);
}

negative_control!(
    lint_wgsl_enable_f16_fails_naming_the_rule,
    "the clean fixture has no `enable f16`, so the rule must not fire",
    expected = "did not fire",
    check_fails_naming(&fixture("clean.wgsl"), Rule::NoEnableF16)
);

#[test]
fn lint_wgsl_ungrouped_r_fails_naming_the_rule() {
    check_fails_naming(&fixture("vec2_groups.wgsl"), Rule::Vec2Groups);
}

negative_control!(
    lint_wgsl_ungrouped_r_fails_naming_the_rule,
    "the clean fixture groups r as vec2, so the rule must not fire",
    expected = "did not fire",
    check_fails_naming(&fixture("clean.wgsl"), Rule::Vec2Groups)
);

#[test]
fn lint_wgsl_inline_word_fails_naming_the_rule() {
    check_fails_naming(&fixture("word_inline.wgsl"), Rule::WordBinding);
}

negative_control!(
    lint_wgsl_inline_word_fails_naming_the_rule,
    "the clean fixture binds the word buffer on its own, so the rule must not fire",
    expected = "did not fire",
    check_fails_naming(&fixture("clean.wgsl"), Rule::WordBinding)
);

/// A commented-out `enable f16` is no directive.
fn check_commented_enable_passes(source: &str) {
    let found = check(source).expect("the WGSL parses and validates");
    assert!(
        found.is_empty(),
        "a commented-out directive fired: {found:?}"
    );
}

#[test]
fn lint_wgsl_commented_enable_f16_is_not_a_directive() {
    let clean = read(&fixture("clean.wgsl"));
    check_commented_enable_passes(&format!("// enable f16;\n/* enable f16; */\n{clean}"));
}

negative_control!(
    lint_wgsl_commented_enable_f16_is_not_a_directive,
    "the same directive uncommented must fire",
    expected = "a commented-out directive fired",
    check_commented_enable_passes(&format!("enable f16;\n{}", read(&fixture("clean.wgsl"))))
);

/// A read of the word buffer at a constant index is not per-copy.
fn check_constant_index_fires(source: &str) {
    let found = check(source).expect("the WGSL parses and validates");
    assert!(
        found
            .iter()
            .any(|f| f.rule == Rule::WordBinding && f.what.contains("sample-index argument")),
        "a constant-index read of the word buffer did not fire: {found:?}"
    );
}

#[test]
fn lint_wgsl_constant_index_word_read_fails() {
    let clean = read(&fixture("clean.wgsl"));
    check_constant_index_fires(&format!(
        "{clean}\nfn first_word() -> vec4<u32> {{ return word_buffer[0]; }}\n"
    ));
}

negative_control!(
    lint_wgsl_constant_index_word_read_fails,
    "the clean fixture reads the word buffer only at its argument, so the rule must not fire",
    expected = "did not fire",
    check_constant_index_fires(&read(&fixture("clean.wgsl")))
);

#[test]
fn lint_wgsl_state_buffer_in_group_0_fails_naming_the_rule() {
    check_fails_naming(&fixture("state_group0.wgsl"), Rule::Bindings);
}

negative_control!(
    lint_wgsl_state_buffer_in_group_0_fails_naming_the_rule,
    "the clean fixture binds both buffers in group 1 at the table's numbers, so the rule must not fire",
    expected = "did not fire",
    check_fails_naming(&fixture("clean.wgsl"), Rule::Bindings)
);

#[test]
fn lint_wgsl_word_buffer_wrong_binding_fails_naming_the_rule() {
    check_fails_naming(&fixture("word_binding_number.wgsl"), Rule::Bindings);
}

negative_control!(
    lint_wgsl_word_buffer_wrong_binding_fails_naming_the_rule,
    "the clean fixture binds the word buffer at WORD_BINDING's 1, so the rule must not fire",
    expected = "did not fire",
    check_fails_naming(&fixture("clean.wgsl"), Rule::Bindings)
);

#[test]
fn lint_wgsl_word_buffer_outside_sample_word_fails_naming_the_rule() {
    check_fails_naming(&fixture("word_outside_sample.wgsl"), Rule::SampleOnly);
}

negative_control!(
    lint_wgsl_word_buffer_outside_sample_word_fails_naming_the_rule,
    "the clean fixture reads each buffer only through its sample function, so the rule must not fire",
    expected = "did not fire",
    check_fails_naming(&fixture("clean.wgsl"), Rule::SampleOnly)
);

/// The read-side `SimState` (lowering Part 3a), appended to the clean fixture under the name `name`: it holds the
/// word it read, `sample.word`, and is never stored.
fn with_read_side(name: &str) -> String {
    format!(
        "{}\nstruct {name} {{\n    r: array<vec2<f32>, 3>,\n    p: array<vec2<f32>, 3>,\n    word: vec4<u32>,\n}}\n\
         fn read_word(i: u32) -> {name} {{ var out: {name}; out.word = sample_word(i); return out; }}\n",
        read(&fixture("clean.wgsl"))
    )
}

/// The lint finds nothing in `source`.
fn check_source_passes(source: &str) {
    let found = check(source).expect("the WGSL parses and validates");
    assert!(found.is_empty(), "lint wgsl finding(s): {found:?}");
}

#[test]
fn lint_wgsl_read_side_simstate_may_hold_the_word() {
    check_source_passes(&with_read_side("SimState"));
}

negative_control!(
    lint_wgsl_read_side_simstate_may_hold_the_word,
    "the same struct under a stored layout's name is a word inside SimState, so the rule must fire",
    expected = "lint wgsl finding(s)",
    check_source_passes(&with_read_side("SimStateBase"))
);

fn generated_file(rel: &str) -> String {
    read(&Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(rel))
}

/// The lint finds nothing in the generated read side, linted as the generated layer's continuation.
fn check_read_side_passes(read_side: &str) {
    let found = check_read_side(&generated_file(GENERATED), read_side)
        .expect("the layer and read side parse and validate");
    assert!(
        found.is_empty(),
        "lint wgsl finding(s) in the read side: {found:?}"
    );
}

#[test]
fn lint_wgsl_passes_on_the_generated_read_side() {
    check_read_side_passes(&generated_file(READ_SIDE_FILE));
}

/// A function named `isnan` and a call to it: the isinf-isnan rule fires on the call (R-351).
const ISNAN_CALL: &str =
    "fn isnan(x: f32) -> bool { return (bitcast<u32>(x) & 0x7fffffffu) > 0x7f800000u; }\n\
                          fn bad(x: f32) -> bool { return isnan(x); }\n";
/// The same rule's finding in the layer: a function named `isinf` and a call to it.
const ISINF_CALL: &str = "fn isinf(x: f32) -> bool { return x > 1.0; }\nfn layer_bad(x: f32) -> bool { return isinf(x); }\n";
/// A struct under a stored layout's name holding a word: the word-binding rule fires, with no line.
const WORD_STRUCT: &str =
    "struct SimStateWord { r: array<vec2<f32>, 3>, p: array<vec2<f32>, 3>, w: vec4<u32> }\n";

negative_control!(
    lint_wgsl_passes_on_the_generated_read_side,
    "a read side that tests a value with isnan has a finding",
    expected = "lint wgsl finding(s) in the read side",
    check_read_side_passes(&format!("{}\n{ISNAN_CALL}", generated_file(READ_SIDE_FILE)))
);

/// The layer with `layer_extra`, then the read side with `source_extra`, the `isnan` call and the word struct: the
/// read side's findings are exactly the struct's, with no line, and the call's, at its own line of the read side; the
/// layer's own findings are not repeated.
fn check_read_side_findings(layer_extra: &str, source_extra: &str) {
    let layer = format!("{}{layer_extra}", generated_file(GENERATED));
    let source = format!(
        "{}\n{source_extra}{ISNAN_CALL}{WORD_STRUCT}",
        generated_file(READ_SIDE_FILE)
    );
    let call_line = u32::try_from(source.lines().count() - 1).expect("a line number");
    let found = check_read_side(&layer, &source).expect("parses and validates");
    let lines: Vec<(Rule, Option<u32>)> = found.iter().map(|f| (f.rule, f.line)).collect();
    assert_eq!(
        lines,
        [(Rule::WordBinding, None), (Rule::IsInfNan, Some(call_line))],
        "the read side's findings and their lines: {found:?}"
    );
}

#[test]
fn lint_wgsl_read_side_findings_at_its_own_lines() {
    check_read_side_findings(ISINF_CALL, "");
}

negative_control!(
    lint_wgsl_read_side_findings_at_its_own_lines,
    "the same isinf call in the read side is the read side's finding",
    expected = "the read side's findings and their lines",
    check_read_side_findings("", ISINF_CALL)
);

#[test]
fn lint_wgsl_read_side_findings_after_a_layer_without_a_final_newline() {
    check_read_side_findings("fn more() {}", "");
}

negative_control!(
    lint_wgsl_read_side_findings_after_a_layer_without_a_final_newline,
    "an isinf call in the read side is one more finding",
    expected = "the read side's findings and their lines",
    check_read_side_findings("fn more() {}", ISINF_CALL)
);
