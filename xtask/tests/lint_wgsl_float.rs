//! `cargo xtask lint wgsl`'s float rules (REQ-RENDER-083; R-351, R-352, R-343, R-297). Each fixture under
//! `tests/fixtures/lint_wgsl/frag/` is linted as a hand-written file under `crates/render/frag/` of a scratch
//! workspace, beside a clean generated file, through [`lint`], the function `cargo xtask lint wgsl` runs. A breaking
//! fixture fails it, naming the file, the line (the fixture's line marked `// fires`), the rule and the bit-pattern
//! fix; each clean fixture, which holds the near misses an over-broad rule would fail, passes. Each test has a
//! registered negative control (R-176): a fires test's control runs the same check on its category's clean fixture, a
//! clean test's on a breaking fixture.
//!
//! - `lint_wgsl_unset_*`: `isinf`, `isnan`, an inf constant, a NaN constant, and a file outside `generated/` linted.
//! - `lint_wgsl_self_compare_*`: a float compared with itself by each of the six operators, a `let`, a buffer element
//!   read twice, a `vec2<f32>`, a variable read twice; and the structural reading of "itself", expression by
//!   expression, with a near miss for each.
//! - `lint_wgsl_finite_max_*`: 65504 and 3.40282347e38 in each spelling, negated, as a `bitcast<f32>` or a module
//!   constant, on the left, and by each operator; and the constant expressions that evaluate to a stand-in, inf or NaN.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use validation::negative_control;
use xtask::lint_wgsl::{
    check, check_fragment, check_fragment_module, lint, run, Rule, BIT_PATTERN_FIX, FRAG_DIR,
    GENERATED,
};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/lint_wgsl")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The fragment fixture `name` (no extension).
fn frag(name: &str) -> String {
    read(&fixtures().join("frag").join(format!("{name}.wgsl")))
}

/// A scratch workspace of its own: a `Cargo.toml`, the clean fixture as the generated file, and `files` (each a path
/// under `crates/render/frag/` and its source).
fn workspace(files: &[(&str, &str)]) -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "lint_wgsl_float_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    let write = |rel: &str, text: &str| {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
    };
    write("Cargo.toml", "");
    write(GENERATED, &read(&fixtures().join("clean.wgsl")));
    for (rel, text) in files {
        write(&format!("{FRAG_DIR}/{rel}"), text);
    }
    root
}

/// The line of `source` marked `// fires`, 1-based.
fn marked_line(source: &str) -> u32 {
    let i = source
        .lines()
        .position(|l| l.contains("// fires"))
        .expect("the fixture marks its line with `// fires`");
    u32::try_from(i + 1).unwrap()
}

/// The fixture `name`, linted as `crates/render/frag/<name>.wgsl`, fails with a finding of `rule` naming the file,
/// the marked line, the rule and the bit-pattern fix.
fn check_fires(name: &str, rule: Rule) {
    let source = frag(name);
    let rel = format!("{name}.wgsl");
    let root = workspace(&[(&rel, &source)]);
    let reports = lint(&root).expect("the files parse and validate");
    let _ = std::fs::remove_dir_all(&root);
    let file = format!("{FRAG_DIR}/{rel}");
    let report = reports
        .iter()
        .find(|r| r.file == file)
        .unwrap_or_else(|| panic!("{file} was not linted: {reports:?}"));
    let lines: Vec<String> = report.findings.iter().map(|f| f.at(&file)).collect();
    let Some(line) = source.contains("// fires").then(|| marked_line(&source)) else {
        assert!(
            report.findings.iter().any(|f| f.rule == rule),
            "[{rule}] did not fire on {file}: {lines:?}"
        );
        panic!("[{rule}] fired on {file}, which marks no line: {lines:?}");
    };
    let want = format!("{file}:{line}: [{rule}] ");
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with(&want) && l.ends_with(BIT_PATTERN_FIX)),
        "[{rule}] did not fire on {file} at line {line} naming the fix: {lines:?}"
    );
}

/// The fixture `name`, linted as `crates/render/frag/<name>.wgsl`, has no finding, nor has any other file.
fn check_clean(name: &str) {
    let source = frag(name);
    let root = workspace(&[(&format!("{name}.wgsl"), &source)]);
    let reports = lint(&root).expect("the files parse and validate");
    let _ = std::fs::remove_dir_all(&root);
    let lines: Vec<String> = reports
        .iter()
        .flat_map(|r| r.findings.iter().map(|f| f.at(&r.file)))
        .collect();
    assert!(lines.is_empty(), "lint wgsl finding(s): {lines:?}");
}

/// A test that fixture `$fixture` fails naming `$rule`, with its control: the same check on `$clean`, which must not
/// fire.
macro_rules! fires {
    ($test:ident, $fixture:literal, $rule:expr, $clean:literal) => {
        #[test]
        fn $test() {
            check_fires($fixture, $rule);
        }

        negative_control!(
            $test,
            "the category's clean fixture breaks no float rule, so the rule must not fire",
            expected = "did not fire",
            check_fires($clean, $rule)
        );
    };
}

// ---------------------------------------------------------------------------------------------------------------
// isinf, isnan, inf and NaN constants (R-351).

fires!(
    lint_wgsl_unset_isinf_fails,
    "unset_isinf",
    Rule::IsInfNan,
    "unset_clean"
);
fires!(
    lint_wgsl_unset_isnan_fails,
    "unset_isnan",
    Rule::IsInfNan,
    "unset_clean"
);
fires!(
    lint_wgsl_unset_inf_constant_fails,
    "unset_inf_constant",
    Rule::InfNanConstant,
    "unset_clean"
);
fires!(
    lint_wgsl_unset_nan_constant_fails,
    "unset_nan_constant",
    Rule::InfNanConstant,
    "unset_clean"
);

#[test]
fn lint_wgsl_unset_clean_fixture_passes() {
    check_clean("unset_clean");
}

negative_control!(
    lint_wgsl_unset_clean_fixture_passes,
    "a file calling isinf has a finding, so the passing check must fail",
    expected = "lint wgsl finding(s)",
    check_clean("unset_isinf")
);

/// `cargo xtask lint wgsl`'s `run` on a workspace whose hand-written file `rel`, outside `generated/`, holds
/// `source`: it fails, naming the file, and still reports the generated file clean.
fn check_run_fails_naming(rel: &str, source: &str) {
    let root = workspace(&[(rel, source)]);
    let result = run(&root.join("Cargo.toml"));
    let reports = lint(&root).expect("the files parse and validate");
    let _ = std::fs::remove_dir_all(&root);
    let file = format!("{FRAG_DIR}/{rel}");
    assert!(
        reports
            .iter()
            .any(|r| r.file == GENERATED && r.findings.is_empty()),
        "the generated file was not linted clean: {reports:?}"
    );
    let err = result.expect_err("the hand-written file outside generated/ was not linted");
    assert!(
        err.contains(&file),
        "the failure does not name {file}: {err}"
    );
}

#[test]
fn lint_wgsl_unset_hand_written_file_is_linted() {
    check_run_fails_naming("shade/unset.wgsl", &frag("unset_isinf"));
}

negative_control!(
    lint_wgsl_unset_hand_written_file_is_linted,
    "a clean hand-written file has no finding, so the run passes",
    expected = "was not linted",
    check_run_fails_naming("shade/unset.wgsl", &frag("unset_clean"))
);

// ---------------------------------------------------------------------------------------------------------------
// A float compared with itself (R-352).

fires!(
    lint_wgsl_self_compare_ne_fails,
    "self_ne",
    Rule::SelfCompare,
    "self_clean"
);
fires!(
    lint_wgsl_self_compare_eq_fails,
    "self_eq",
    Rule::SelfCompare,
    "self_clean"
);
fires!(
    lint_wgsl_self_compare_lt_fails,
    "self_lt",
    Rule::SelfCompare,
    "self_clean"
);
fires!(
    lint_wgsl_self_compare_le_fails,
    "self_le",
    Rule::SelfCompare,
    "self_clean"
);
fires!(
    lint_wgsl_self_compare_gt_fails,
    "self_gt",
    Rule::SelfCompare,
    "self_clean"
);
fires!(
    lint_wgsl_self_compare_ge_fails,
    "self_ge",
    Rule::SelfCompare,
    "self_clean"
);
fires!(
    lint_wgsl_self_compare_let_fails,
    "self_let",
    Rule::SelfCompare,
    "self_clean"
);
fires!(
    lint_wgsl_self_compare_buffer_element_fails,
    "self_buffer",
    Rule::SelfCompare,
    "self_clean"
);
fires!(
    lint_wgsl_self_compare_vec2_fails,
    "self_vec2",
    Rule::SelfCompare,
    "self_clean"
);
fires!(
    lint_wgsl_self_compare_variable_read_twice_fails,
    "self_var",
    Rule::SelfCompare,
    "self_clean"
);

#[test]
fn lint_wgsl_self_compare_clean_fixture_passes() {
    check_clean("self_clean");
}

negative_control!(
    lint_wgsl_self_compare_clean_fixture_passes,
    "a file comparing a float with itself has a finding, so the passing check must fail",
    expected = "lint wgsl finding(s)",
    check_clean("self_ne")
);

// ---------------------------------------------------------------------------------------------------------------
// Finite-max stand-ins (R-352).

fires!(
    lint_wgsl_finite_max_65504_0_fails,
    "finite_max_65504_0",
    Rule::FiniteMax,
    "finite_max_clean"
);
fires!(
    lint_wgsl_finite_max_65504_dot_fails,
    "finite_max_65504_dot",
    Rule::FiniteMax,
    "finite_max_clean"
);
fires!(
    lint_wgsl_finite_max_6_5504e4_fails,
    "finite_max_6_5504e4",
    Rule::FiniteMax,
    "finite_max_clean"
);
fires!(
    lint_wgsl_finite_max_65504_0f_fails,
    "finite_max_65504_0f",
    Rule::FiniteMax,
    "finite_max_clean"
);
fires!(
    lint_wgsl_finite_max_65504h_fails,
    "finite_max_65504h",
    Rule::FiniteMax,
    "finite_max_clean"
);
fires!(
    lint_wgsl_finite_max_negative_65504_fails,
    "finite_max_neg_65504",
    Rule::FiniteMax,
    "finite_max_clean"
);
fires!(
    lint_wgsl_finite_max_f32_fails,
    "finite_max_f32",
    Rule::FiniteMax,
    "finite_max_clean"
);
fires!(
    lint_wgsl_finite_max_f32_short_spelling_fails,
    "finite_max_f32_short",
    Rule::FiniteMax,
    "finite_max_clean"
);
fires!(
    lint_wgsl_finite_max_negative_f32_fails,
    "finite_max_neg_f32",
    Rule::FiniteMax,
    "finite_max_clean"
);
fires!(
    lint_wgsl_finite_max_bitcast_f16_bits_fails,
    "finite_max_bitcast_f16",
    Rule::FiniteMax,
    "finite_max_clean"
);
fires!(
    lint_wgsl_finite_max_bitcast_f32_bits_fails,
    "finite_max_bitcast_f32",
    Rule::FiniteMax,
    "finite_max_clean"
);
fires!(
    lint_wgsl_finite_max_module_constant_fails,
    "finite_max_constant",
    Rule::FiniteMax,
    "finite_max_clean"
);
fires!(
    lint_wgsl_finite_max_constant_on_the_left_fails,
    "finite_max_left",
    Rule::FiniteMax,
    "finite_max_clean"
);
fires!(
    lint_wgsl_finite_max_lt_fails,
    "finite_max_lt",
    Rule::FiniteMax,
    "finite_max_clean"
);
fires!(
    lint_wgsl_finite_max_le_fails,
    "finite_max_le",
    Rule::FiniteMax,
    "finite_max_clean"
);
fires!(
    lint_wgsl_finite_max_gt_fails,
    "finite_max_gt",
    Rule::FiniteMax,
    "finite_max_clean"
);
fires!(
    lint_wgsl_finite_max_ge_fails,
    "finite_max_ge",
    Rule::FiniteMax,
    "finite_max_clean"
);
fires!(
    lint_wgsl_finite_max_eq_fails,
    "finite_max_eq",
    Rule::FiniteMax,
    "finite_max_clean"
);
fires!(
    lint_wgsl_finite_max_ne_fails,
    "finite_max_ne",
    Rule::FiniteMax,
    "finite_max_clean"
);

#[test]
fn lint_wgsl_finite_max_clean_fixture_passes() {
    check_clean("finite_max_clean");
}

negative_control!(
    lint_wgsl_finite_max_clean_fixture_passes,
    "a file comparing against 65504.0 has a finding, so the passing check must fail",
    expected = "lint wgsl finding(s)",
    check_clean("finite_max_65504_0")
);

/// `source`, checked as the generated file, has findings of every rule in `rules`.
fn check_generated_names(source: &str, rules: &[Rule]) {
    let found = check(source).expect("the WGSL parses and validates");
    for rule in rules {
        assert!(
            found.iter().any(|f| f.rule == *rule),
            "[{rule}] is not among the findings: {found:?}"
        );
    }
}

/// The `65504h` fixture as the generated file trips TASK-M0-13's "no `enable f16`" rule too; the finite-max rule is
/// among its findings beside it.
#[test]
fn lint_wgsl_finite_max_65504h_as_generated_names_the_rule() {
    check_generated_names(
        &frag("finite_max_65504h"),
        &[Rule::FiniteMax, Rule::NoEnableF16],
    );
}

negative_control!(
    lint_wgsl_finite_max_65504h_as_generated_names_the_rule,
    "an f16 file with no finite-max comparison trips only the f16 rule",
    expected = "[finite-max] is not among the findings",
    check_generated_names(
        "enable f16;\nfn half(d: f16) -> bool { return d > 1.0h; }\n",
        &[Rule::FiniteMax, Rule::NoEnableF16],
    )
);

// ---------------------------------------------------------------------------------------------------------------
// The rules expression by expression, on inline sources: each case is one function body's `return` expression.

/// `expr` as the return of a function over floats `x`, `y`, ints `i`, `j`, a vector `p`, a bool `c` and an f16 `h`.
fn wrap(expr: &str) -> String {
    format!(
        "enable f16;\nfn f(x: f32, y: f32, i: i32, j: i32, p: vec4<f32>, c: bool, h: f16) -> bool {{\n    \
         return {expr};\n}}\n"
    )
}

/// Each case in `cases` breaks `rule` if `fires`, and does not if not.
fn check_cases(cases: &[&str], rule: Rule, fires: bool) {
    for case in cases {
        let found = check_fragment(&wrap(case)).expect("the case parses and validates");
        let hit = found.iter().any(|f| f.rule == rule && f.line == Some(3));
        if fires {
            assert!(hit, "[{rule}] did not fire on `{case}`: {found:?}");
        } else {
            assert!(!hit, "[{rule}] fired on `{case}`, a near miss: {found:?}");
        }
    }
}

/// Structurally equal operands, each built of a different kind of expression.
const SELF_SAME: [&str; 12] = [
    "(x + 1.0) != (x + 1.0)",
    "-x != -x",
    "abs(x) != abs(x)",
    "clamp(x, 0.0, 1.0) != clamp(x, 0.0, 1.0)",
    "fma(x, y, 1.0) != fma(x, y, 1.0)",
    "all(p.xy != p.xy)",
    "p.z != p.z",
    "all(vec2(x) != vec2(x))",
    "all(vec2(x, y) != vec2(x, y))",
    "f32(i) != f32(i)",
    "bitcast<f32>(i) != bitcast<f32>(i)",
    "select(x, y, c) != select(x, y, c)",
];

/// Each a near miss of a `SELF_SAME` case: the operands differ in one part.
const SELF_DIFFERENT: [&str; 14] = [
    "(x + 1.0) != (x + 2.0)",
    "(x + 1.0) != (x - 1.0)",
    "-x != -y",
    "abs(x) != sqrt(x)",
    "clamp(x, 0.0, 1.0) != clamp(x, 0.0, 2.0)",
    "fma(x, y, 1.0) != fma(x, y, 2.0)",
    "all(p.xy != p.yx)",
    "p.z != p.w",
    "all(vec2(x) != vec2(y))",
    "all(vec2(x, y) != vec2(y, x))",
    "f32(i) != f32(j)",
    "bitcast<f32>(i) != f32(i)",
    "select(x, y, c) != select(y, x, c)",
    "x < y",
];

#[test]
fn lint_wgsl_self_compare_structurally_equal_operands_fire() {
    check_cases(&SELF_SAME, Rule::SelfCompare, true);
}

negative_control!(
    lint_wgsl_self_compare_structurally_equal_operands_fire,
    "operands that differ are no self-comparison",
    expected = "did not fire",
    check_cases(&SELF_DIFFERENT, Rule::SelfCompare, true)
);

#[test]
fn lint_wgsl_self_compare_differing_operands_do_not_fire() {
    check_cases(&SELF_DIFFERENT, Rule::SelfCompare, false);
}

negative_control!(
    lint_wgsl_self_compare_differing_operands_do_not_fire,
    "structurally equal operands are a self-comparison",
    expected = "a near miss",
    check_cases(&SELF_SAME, Rule::SelfCompare, false)
);

/// Constant expressions that evaluate to a finite-max stand-in, beyond the literal spellings of the fixtures.
const FINITE_MAX_EXPRESSIONS: [&str; 6] = [
    "all(vec2(x, y) < vec2(65504.0))",
    "all(vec2(x, y) < vec2(1.0, 65504.0))",
    "x < f32(65504h)",
    "h > -(-65504h)",
    "all(vec2(x, y) < bitcast<vec2<f32>>(vec2(0x477fe000u)))",
    "x < bitcast<f32>(2139095039i)",
];

/// Each a near miss of a `FINITE_MAX_EXPRESSIONS` case: a value below the maximum (65472, the f16 below 65504), or no
/// constant.
const FINITE_MAX_NEAR: [&str; 6] = [
    "all(vec2(x, y) < vec2(65503.0))",
    "all(vec2(x, y) < vec2(1.0, y))",
    "x < f32(65472h)",
    "h > -(-65472h)",
    "all(vec2(x, y) < bitcast<vec2<f32>>(vec2(0x477fdfffu)))",
    "x < bitcast<f32>(i)",
];

#[test]
fn lint_wgsl_finite_max_constant_expressions_fire() {
    check_cases(&FINITE_MAX_EXPRESSIONS, Rule::FiniteMax, true);
}

negative_control!(
    lint_wgsl_finite_max_constant_expressions_fire,
    "values below the maximum are no stand-in",
    expected = "did not fire",
    check_cases(&FINITE_MAX_NEAR, Rule::FiniteMax, true)
);

#[test]
fn lint_wgsl_finite_max_near_misses_do_not_fire() {
    check_cases(&FINITE_MAX_NEAR, Rule::FiniteMax, false);
}

negative_control!(
    lint_wgsl_finite_max_near_misses_do_not_fire,
    "the maximum itself is a stand-in",
    expected = "a near miss",
    check_cases(&FINITE_MAX_EXPRESSIONS, Rule::FiniteMax, false)
);

/// Constant expressions that evaluate to inf or NaN: -inf, a vector, a module constant's bits.
const INF_NAN_EXPRESSIONS: [&str; 3] = [
    "x > bitcast<f32>(0xff800000u)",
    "all(vec2(x, y) != bitcast<vec2<f32>>(vec2(0x7f800000u, 0u)))",
    "x != bitcast<f32>(BITS)",
];

/// Each a near miss of an `INF_NAN_EXPRESSIONS` case: a finite bit pattern.
const INF_NAN_NEAR: [&str; 3] = [
    "x > bitcast<f32>(0xff7fffffu)",
    "all(vec2(x, y) != bitcast<vec2<f32>>(vec2(0x3f800000u, 0u)))",
    "x != bitcast<f32>(ONE_BITS)",
];

/// Each case with the module constants `BITS` (a NaN) and `ONE_BITS` (1.0) declared.
fn check_inf_nan_cases(cases: &[&str], fires: bool) {
    for case in cases {
        let source = format!(
            "{}const BITS: u32 = 0x7fc00001u;\nconst ONE_BITS: u32 = 0x3f800000u;\n",
            wrap(case)
        );
        let found = check_fragment(&source).expect("the case parses and validates");
        let hit = found.iter().any(|f| f.rule == Rule::InfNanConstant);
        if fires {
            assert!(
                hit,
                "[inf-nan-constant] did not fire on `{case}`: {found:?}"
            );
        } else {
            assert!(
                !hit,
                "[inf-nan-constant] fired on `{case}`, a near miss: {found:?}"
            );
        }
    }
}

#[test]
fn lint_wgsl_unset_inf_nan_constant_expressions_fire() {
    check_inf_nan_cases(&INF_NAN_EXPRESSIONS, true);
}

negative_control!(
    lint_wgsl_unset_inf_nan_constant_expressions_fire,
    "finite bit patterns are no inf or NaN",
    expected = "did not fire",
    check_inf_nan_cases(&INF_NAN_NEAR, true)
);

#[test]
fn lint_wgsl_unset_finite_bit_patterns_do_not_fire() {
    check_inf_nan_cases(&INF_NAN_NEAR, false);
}

negative_control!(
    lint_wgsl_unset_finite_bit_patterns_do_not_fire,
    "inf and NaN bit patterns are inf and NaN constants",
    expected = "a near miss",
    check_inf_nan_cases(&INF_NAN_EXPRESSIONS, false)
);

/// naga's own `IsNan` (as another front end would give it): `!(x > 0.0)` with its negation rewritten to
/// `IsNan(x)`, when `nan`; left as it is otherwise. The rule must fire, naming line 2.
fn check_naga_is_nan(nan: bool) {
    let source = "fn f(x: f32) -> bool {\n    return !(x > 0.0);\n}\n";
    let mut module = naga::front::wgsl::parse_str(source).expect("the source parses");
    let (_, function) = module.functions.iter_mut().next().expect("one function");
    let (not, x) = function
        .expressions
        .iter()
        .find_map(|(h, e)| match *e {
            naga::Expression::Unary { expr, .. } => match function.expressions[expr] {
                naga::Expression::Binary { left, .. } => Some((h, left)),
                _ => None,
            },
            _ => None,
        })
        .expect("the negated comparison");
    if nan {
        function.expressions[not] = naga::Expression::Relational {
            fun: naga::RelationalFunction::IsNan,
            argument: x,
        };
    }
    let found = check_fragment_module(&module, source).expect("the module validates");
    assert!(
        found
            .iter()
            .any(|f| f.rule == Rule::IsInfNan && f.line == Some(2)),
        "[isinf-isnan] did not fire on naga's IsNan: {found:?}"
    );
}

#[test]
fn lint_wgsl_unset_naga_is_nan_fails() {
    check_naga_is_nan(true);
}

negative_control!(
    lint_wgsl_unset_naga_is_nan_fails,
    "a plain negation is no IsNan",
    expected = "did not fire",
    check_naga_is_nan(false)
);
