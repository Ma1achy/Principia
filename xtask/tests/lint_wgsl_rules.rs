//! `cargo xtask lint wgsl`'s rules in detail (REQ-RENDER-001, R-343; R-196's mutation gate): each rule's name as a
//! finding shows it; `strip_comments`, which the `enable f16` rule reads, on block, nested and line comments; an f16
//! type; `r` grouped as three f32s or two vec2s; a `SimState*` struct with no `p`; a word buffer of fixed size or of
//! `vec2<u32>`; the type each wrongly typed buffer is told it should be; and the `ci` registry's `lint wgsl` runner
//! running the lint.

use std::path::Path;
use std::process::Command;

use validation::negative_control;
use xtask::lint_wgsl::{check, strip_comments, Finding, Rule};

fn clean() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/lint_wgsl/clean.wgsl");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// `source` with `from` replaced by `to`, which must occur in it.
fn edited(source: &str, from: &str, to: &str) -> String {
    assert!(source.contains(from), "the fixture has no `{from}`");
    source.replace(from, to)
}

fn findings(source: &str) -> Vec<Finding> {
    check(source).expect("the WGSL parses and validates")
}

/// `source` has a finding of `rule` whose text contains `what`.
fn check_fires(source: &str, rule: Rule, what: &str) {
    let found = findings(source);
    assert!(
        found
            .iter()
            .any(|f| f.rule == rule && f.what.contains(what)),
        "[{rule}] did not fire with {what:?}: {found:?}"
    );
}

// ---------------------------------------------------------------------------------------------------------------
// Each rule's name.

const NAMES: [(Rule, &str); 7] = [
    (Rule::ExtractBitsU32, "extractBits-u32"),
    (Rule::NoF64, "no-f64"),
    (Rule::NoEnableF16, "no-enable-f16"),
    (Rule::Vec2Groups, "vec2-groups"),
    (Rule::WordBinding, "word-binding"),
    (Rule::Bindings, "bindings"),
    (Rule::SampleOnly, "sample-only"),
];

fn check_names(names: &[(Rule, &str)]) {
    for (rule, name) in names {
        assert_eq!(
            rule.to_string(),
            *name,
            "{rule:?} is displayed as another name"
        );
    }
}

#[test]
fn lint_wgsl_each_rule_displays_its_name() {
    check_names(&NAMES);
}

negative_control!(
    lint_wgsl_each_rule_displays_its_name,
    "`bindings` is not displayed as `binding`",
    expected = "is displayed as another name",
    check_names(&[(Rule::Bindings, "binding")])
);

// ---------------------------------------------------------------------------------------------------------------
// strip_comments.

/// Each comment is blanked to spaces, a newline in it kept; a nested block comment closes only at its outer `*/`; a
/// lone `/` or `*`, or a `*/` outside a comment, is code.
const STRIPPED: [(&str, &str); 6] = [
    ("a/*b*/c", "a   c"),
    ("a/*x/*y*/z*/c", "a       c"),
    ("/*\n*/x", " \n x"),
    ("a // b\nc", "a \nc"),
    ("a / b * c", "a / b * c"),
    ("a*/b", "a*/b"),
];

fn check_stripped(cases: &[(&str, &str)]) {
    for (source, want) in cases {
        assert_eq!(strip_comments(source), *want, "strip_comments({source:?})");
    }
}

#[test]
fn lint_wgsl_strip_comments_blanks_block_nested_and_line_comments() {
    check_stripped(&STRIPPED);
}

negative_control!(
    lint_wgsl_strip_comments_blanks_block_nested_and_line_comments,
    "a block comment is not kept",
    expected = "strip_comments(\"a/*b*/c\")",
    check_stripped(&[("a/*b*/c", "a/*b*/c")])
);

// ---------------------------------------------------------------------------------------------------------------
// An f16 type, beside the directive.

#[test]
fn lint_wgsl_f16_type_fires() {
    check_fires(
        &format!(
            "enable f16;\n{}\nfn h(x: f16) -> f16 {{ return x; }}\n",
            clean()
        ),
        Rule::NoEnableF16,
        "an f16 type",
    );
}

negative_control!(
    lint_wgsl_f16_type_fires,
    "the directive alone declares no f16 type",
    expected = "did not fire",
    check_fires(
        &format!("enable f16;\n{}", clean()),
        Rule::NoEnableF16,
        "an f16 type"
    )
);

// ---------------------------------------------------------------------------------------------------------------
// vec2 groups.

const R_VEC2: &str = "    r: array<vec2<f32>, 3>,";

#[test]
fn lint_wgsl_r_as_three_f32_fires() {
    check_fires(
        &edited(&clean(), R_VEC2, "    r: array<f32, 3>,"),
        Rule::Vec2Groups,
        "`SimStateFTLE.r` is not",
    );
}

negative_control!(
    lint_wgsl_r_as_three_f32_fires,
    "the clean fixture groups r as vec2",
    expected = "did not fire",
    check_fires(&clean(), Rule::Vec2Groups, "`SimStateFTLE.r` is not")
);

#[test]
fn lint_wgsl_r_as_two_vec2_fires() {
    check_fires(
        &edited(&clean(), R_VEC2, "    r: array<vec2<f32>, 2>,"),
        Rule::Vec2Groups,
        "`SimStateFTLE.r` is not",
    );
}

negative_control!(
    lint_wgsl_r_as_two_vec2_fires,
    "the clean fixture groups r as three vec2s",
    expected = "did not fire",
    check_fires(&clean(), Rule::Vec2Groups, "`SimStateFTLE.r` is not")
);

#[test]
fn lint_wgsl_simstate_without_p_fires() {
    check_fires(
        &edited(&clean(), "    p: array<vec2<f32>, 3>,\n", ""),
        Rule::Vec2Groups,
        "`SimStateFTLE` has no `p`",
    );
}

negative_control!(
    lint_wgsl_simstate_without_p_fires,
    "the clean fixture's SimStateFTLE has p",
    expected = "did not fire",
    check_fires(&clean(), Rule::Vec2Groups, "`SimStateFTLE` has no `p`")
);

// ---------------------------------------------------------------------------------------------------------------
// The word buffer's type.

const WORD_TYPE: &str = "word_buffer: array<vec4<u32>>;";

#[test]
fn lint_wgsl_fixed_size_word_buffer_fires() {
    check_fires(
        &edited(&clean(), WORD_TYPE, "word_buffer: array<vec4<u32>, 4>;"),
        Rule::WordBinding,
        "is not array<vec4<u32>>",
    );
}

negative_control!(
    lint_wgsl_fixed_size_word_buffer_fires,
    "the clean fixture's word buffer is runtime-sized",
    expected = "did not fire",
    check_fires(&clean(), Rule::WordBinding, "is not array<vec4<u32>>")
);

#[test]
fn lint_wgsl_vec2_word_buffer_fires() {
    let source = edited(&clean(), WORD_TYPE, "word_buffer: array<vec2<u32>>;");
    let source = edited(
        &source,
        "fn sample_word(i: u32) -> vec4<u32>",
        "fn sample_word(i: u32) -> vec2<u32>",
    );
    check_fires(&source, Rule::WordBinding, "is not array<vec4<u32>>");
}

negative_control!(
    lint_wgsl_vec2_word_buffer_fires,
    "the clean fixture's word buffer holds vec4<u32>",
    expected = "did not fire",
    check_fires(&clean(), Rule::WordBinding, "is not array<vec4<u32>>")
);

// ---------------------------------------------------------------------------------------------------------------
// The type a wrongly typed buffer is told it should be (R-343).

/// `source` with `buffer` an `array<u32>` and its reader returning a u32.
fn u32_buffer(buffer: &str, element: &str, reader: &str) -> String {
    let source = edited(
        &clean(),
        &format!("{buffer}: array<{element}>;"),
        &format!("{buffer}: array<u32>;"),
    );
    edited(
        &source,
        &format!("fn {reader}(i: u32) -> {element}"),
        &format!("fn {reader}(i: u32) -> u32"),
    )
}

#[test]
fn lint_wgsl_state_buffer_of_u32_is_told_simstate_ftle() {
    check_fires(
        &u32_buffer("simstate_buffer", "SimStateFTLE", "sample_state"),
        Rule::Bindings,
        "`simstate_buffer` is not a storage `array<SimStateFTLE>`",
    );
}

negative_control!(
    lint_wgsl_state_buffer_of_u32_is_told_simstate_ftle,
    "the clean fixture's state buffer is array<SimStateFTLE>",
    expected = "did not fire",
    check_fires(
        &clean(),
        Rule::Bindings,
        "`simstate_buffer` is not a storage `array<SimStateFTLE>`"
    )
);

#[test]
fn lint_wgsl_word_buffer_of_u32_is_told_vec4() {
    check_fires(
        &u32_buffer("word_buffer", "vec4<u32>", "sample_word"),
        Rule::Bindings,
        "`word_buffer` is not a storage `array<vec4<u32>>`",
    );
}

negative_control!(
    lint_wgsl_word_buffer_of_u32_is_told_vec4,
    "the clean fixture's word buffer is array<vec4<u32>>",
    expected = "did not fire",
    check_fires(
        &clean(),
        Rule::Bindings,
        "`word_buffer` is not a storage `array<vec4<u32>>`"
    )
);

// ---------------------------------------------------------------------------------------------------------------
// The `ci` registry's `lint wgsl` runner runs the lint on this workspace.

const CHILD: &str = "XTASK_LINT_WGSL_CI_CHILD";
const RAN: &str = "lint wgsl: crates/render/frag/generated/payload_unpack.wgsl: every rule holds";

fn check_ran(stdout: &str) {
    assert!(
        stdout.contains(RAN),
        "the ci runner `lint wgsl` did not run the lint; its output: {stdout}"
    );
}

/// Run as a child (with `CHILD` set), calls the registry's `lint wgsl` runner; otherwise runs itself as that child,
/// output uncaptured, and checks the lint's report is in its output.
#[test]
fn lint_wgsl_ci_runner_runs_the_lint() {
    if std::env::var_os(CHILD).is_some() {
        let runner = xtask::ci::RUNNERS
            .iter()
            .find(|r| r.name == "lint wgsl")
            .expect("the registry has `lint wgsl`");
        (runner.run)().expect("the lint passes on this workspace");
        return;
    }
    let out = Command::new(std::env::current_exe().expect("the test binary"))
        .args([
            "--exact",
            "lint_wgsl_ci_runner_runs_the_lint",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD, "1")
        .output()
        .expect("the test binary runs");
    assert!(
        out.status.success(),
        "the child failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    check_ran(&String::from_utf8_lossy(&out.stdout));
}

negative_control!(
    lint_wgsl_ci_runner_runs_the_lint,
    "output with no lint report is not the lint having run",
    expected = "did not run the lint",
    check_ran("running 1 test\ntest lint_wgsl_ci_runner_runs_the_lint ... ok\n")
);
