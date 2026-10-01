//! `cargo xtask lint wgsl` (render contract Part 5, "Unpack layer"; REQ-RENDER-001): it passes on the generated WGSL
//! and on a clean fixture, and each fixture that breaks one rule fails it, naming that rule: an i32 `extractBits`, an
//! f64, `enable f16`, an `r` not vec2-grouped, and a word inside `SimState`; and, for R-343's bindings, the state
//! buffer in group 0, the word buffer at the wrong binding number, and `word_buffer` indexed outside `sample_word`.

use std::path::{Path, PathBuf};

use validation::negative_control;
use xtask::lint_wgsl::{check, Rule, GENERATED};

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
