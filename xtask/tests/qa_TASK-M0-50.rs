//! QA tests for TASK-M0-50, written from REQ-RENDER-083 (statement and verify line; R-351, R-352, R-353, R-343,
//! R-297), not from the implementation. In fragment-stage WGSL (every `.wgsl` under `crates/render/frag/`, generated
//! or written by hand), `cargo xtask lint wgsl` must fail, naming the file, the line, the rule and a bit-pattern test
//! as the fix, on: any use of `isinf`/`isnan`; a float comparison against an inf or NaN constant (a constant expression
//! that evaluates to one included); a float scalar or vector compared with itself by any of the six operators (the
//! same expression, or structurally equal reads of the same `let`, argument, variable or buffer element with no store
//! between); and a comparison against a finite-max stand-in, ±65504 or ±3.40282347e38, in any spelling, either operand
//! order, as a `bitcast<f32>` of its bits or any other constant expression that evaluates to it.
//!
//! Each case is a small WGSL source whose breaking line is marked `// fires`; the tests assert the rule named, that
//! line, and the fix. The near misses, which an over-broad rule would fail, assert no float-rule finding. Each test
//! has a registered negative control (R-176): the fires tests run on their near misses, and the near-miss tests on
//! breaking cases.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use validation::negative_control;
use xtask::lint_wgsl::{
    check, check_fragment, check_fragment_module, lint, run, Rule, FRAG_DIR, GENERATED,
};

const FLOAT_RULES: [Rule; 4] = [
    Rule::IsInfNan,
    Rule::InfNanConstant,
    Rule::SelfCompare,
    Rule::FiniteMax,
];

/// The 1-based line of `src` marked `// fires`.
fn marked(src: &str) -> u32 {
    let i = src
        .lines()
        .position(|l| l.contains("// fires"))
        .unwrap_or_else(|| panic!("case marks no line:\n{src}"));
    u32::try_from(i + 1).unwrap()
}

/// `src`, linted as a hand-written fragment file, has a finding of `rule` at its marked line, whose text names the
/// rule and a bit-pattern test as the fix (R-343).
fn fires(src: &str, rule: Rule) {
    let line = marked(src);
    let found =
        check_fragment(src).unwrap_or_else(|e| panic!("case does not parse/validate: {e}\n{src}"));
    let hit = found
        .iter()
        .find(|f| f.rule == rule && f.line == Some(line));
    let Some(f) = hit else {
        panic!("rule {rule} did not fire at line {line}: {found:?}\n{src}");
    };
    let text = f.at("crates/render/frag/case.wgsl");
    assert!(
        text.starts_with(&format!("crates/render/frag/case.wgsl:{line}: [{rule}]")),
        "the finding does not name file, line and rule: {text}"
    );
    let lower = text.to_lowercase();
    assert!(
        lower.contains("bit pattern") && text.contains("R-343"),
        "the finding does not name a bit-pattern test (R-343) as the fix: {text}"
    );
}

/// `src` has no float-rule finding.
fn quiet(src: &str) {
    let found =
        check_fragment(src).unwrap_or_else(|e| panic!("case does not parse/validate: {e}\n{src}"));
    let float: Vec<_> = found
        .iter()
        .filter(|f| FLOAT_RULES.contains(&f.rule))
        .collect();
    assert!(
        float.is_empty(),
        "float rule finding(s) on a near miss: {float:?}\n{src}"
    );
}

fn all_fire(cases: &[&str], rule: Rule) {
    for c in cases {
        fires(c, rule);
    }
}

fn all_quiet(cases: &[&str]) {
    for c in cases {
        quiet(c);
    }
}

// ---------------------------------------------------------------------------------------------------------------
// isinf / isnan and inf or NaN constants (R-351).

const UNSET_FIRES_ISINF_ISNAN: [&str; 3] = [
    // isnan called from inside a loop and a switch, in a helper called by an entry point.
    "fn isnan(x: f32) -> bool { return (bitcast<u32>(x) & 0x7fffffffu) > 0x7f800000u; }\n\
     fn g(d: f32, k: i32) -> f32 {\n\
     \x20   var acc = 0.0;\n\
     \x20   loop {\n\
     \x20       switch k {\n\
     \x20           case 1: {\n\
     \x20               if isnan(d) { acc = 1.0; } // fires\n\
     \x20           }\n\
     \x20           default: {}\n\
     \x20       }\n\
     \x20       break;\n\
     \x20   }\n\
     \x20   return acc;\n\
     }\n\
     @fragment fn fs() -> @location(0) vec4<f32> { return vec4<f32>(g(1.0, 1)); }\n",
    // isinf called in a let initialiser.
    "fn isinf(x: f32) -> bool { return (bitcast<u32>(x) & 0x7fffffffu) == 0x7f800000u; }\n\
     fn g(d: f32) -> f32 {\n\
     \x20   let unset = isinf(d); // fires\n\
     \x20   return select(d, 0.0, unset);\n\
     }\n",
    // isinf taking a vector.
    "fn isinf(x: vec2<f32>) -> vec2<bool> { return (bitcast<vec2<u32>>(x) & vec2<u32>(0x7fffffffu)) == vec2<u32>(0x7f800000u); }\n\
     fn g(d: vec2<f32>) -> bool {\n\
     \x20   let u = isinf(d); // fires\n\
     \x20   return any(u);\n\
     }\n",
];

const UNSET_FIRES_CONSTANT: [&str; 9] = [
    // +inf from an abstract-int bit pattern.
    "fn g(d: f32) -> bool {\n    return d == bitcast<f32>(0x7f800000); // fires\n}\n",
    // -inf bit pattern, `<=`.
    "fn g(d: f32) -> bool {\n    return d <= bitcast<f32>(0xff800000u); // fires\n}\n",
    // A negated +inf, constant on the left.
    "fn g(d: f32) -> bool {\n    return -bitcast<f32>(0x7f800000u) < d; // fires\n}\n",
    // A quiet NaN through a module constant, `!=`.
    "const QNAN_BITS: u32 = 0x7fc00000u;\n\
     fn g(d: f32) -> bool {\n    return d != bitcast<f32>(QNAN_BITS); // fires\n}\n",
    // inf through a let.
    "fn g(d: f32) -> bool {\n    let INF = bitcast<f32>(0x7f800000u);\n    return d >= INF; // fires\n}\n",
    // A vector against a splatted inf.
    "fn g(d: vec2<f32>) -> vec2<bool> {\n    return d == vec2<f32>(bitcast<f32>(0x7f800000u)); // fires\n}\n",
    // A vector bitcast of inf bits.
    "fn g(d: vec2<f32>) -> vec2<bool> {\n    return d > bitcast<vec2<f32>>(vec2<u32>(0x7f800000u)); // fires\n}\n",
    // A chain of module constants that evaluates to a NaN.
    "const HI: u32 = 0x7f000000u;\nconst NAN_BITS: u32 = HI | 0x00ffffffu;\n\
     fn g(d: f32) -> bool {\n    return bitcast<f32>(NAN_BITS) == d; // fires\n}\n",
    // A constant expression over inf's bitcast that evaluates to inf.
    "fn g(d: f32) -> bool {\n    return d == bitcast<f32>(0x7f800000u) * 1.0; // fires\n}\n",
];

#[test]
fn qa_lint_wgsl_unset_isinf_isnan_fire() {
    all_fire(&UNSET_FIRES_ISINF_ISNAN, Rule::IsInfNan);
}

negative_control!(
    qa_lint_wgsl_unset_isinf_isnan_fire,
    "a bit-pattern unset test calls no isinf/isnan, so the rule must not fire",
    expected = "did not fire",
    all_fire(&UNSET_QUIET, Rule::IsInfNan)
);

#[test]
fn qa_lint_wgsl_unset_inf_nan_constants_fire() {
    all_fire(&UNSET_FIRES_CONSTANT, Rule::InfNanConstant);
}

negative_control!(
    qa_lint_wgsl_unset_inf_nan_constants_fire,
    "comparisons against finite bit patterns and integer bit tests hold no inf/NaN constant",
    expected = "did not fire",
    all_fire(&UNSET_QUIET, Rule::InfNanConstant)
);

/// The near misses: unset tests by bits (the fix itself), integer comparisons against inf's bit pattern, and a float
/// against a finite neighbour of inf's bits. Each marks a line so the controls can run `fires` on them.
const UNSET_QUIET: [&str; 4] = [
    "const PA_D_MIN_UNSET: u32 = 0x7c00u;\n\
     fn g(w: u32) -> bool {\n    return extractBits(w, 16u, 16u) == PA_D_MIN_UNSET; // fires\n}\n",
    "fn g(d: f32) -> bool {\n    return (bitcast<u32>(d) & 0x7fffffffu) == 0x7f800000u; // fires\n}\n",
    "fn g(d: f32) -> bool {\n    return (bitcast<u32>(d) & 0x7fffffffu) > 0x7f800000u; // fires\n}\n",
    "fn g(d: f32) -> bool {\n    return d < bitcast<f32>(0x7f000000u); // fires\n}\n",
];

#[test]
fn qa_lint_wgsl_unset_bit_tests_are_quiet() {
    all_quiet(&UNSET_QUIET);
}

negative_control!(
    qa_lint_wgsl_unset_bit_tests_are_quiet,
    "comparisons against inf/NaN constants are float-rule findings",
    expected = "float rule finding(s)",
    all_quiet(&UNSET_FIRES_CONSTANT)
);

/// naga's IR `IsInf`/`IsNan` (from front ends other than WGSL's) is a finding: a parsed `all(...)` is rewritten into
/// `isInf(x)`/`isNan(x)` (or left alone, for the control).
fn naga_relational(rewrite: bool) {
    let source = "fn f(x: f32) -> bool {\n    return all(vec2<bool>(x > 0.0, true));\n}\n";
    let module = naga::front::wgsl::parse_str(source).expect("parses");
    let (_, function) = module.functions.iter().next().expect("one function");
    let arg = function
        .expressions
        .iter()
        .find(|(_, e)| matches!(e, naga::Expression::FunctionArgument(0)))
        .map(|(h, _)| h)
        .expect("the argument");
    let mut handles = Vec::new();
    for (h, e) in function.expressions.iter() {
        if matches!(e, naga::Expression::Relational { .. }) {
            handles.push(h);
        }
    }
    assert_eq!(handles.len(), 1, "one relational expression");
    for (i, fun) in [
        naga::RelationalFunction::IsInf,
        naga::RelationalFunction::IsNan,
    ]
    .into_iter()
    .enumerate()
    {
        let mut m = module.clone();
        if rewrite {
            let (_, f) = m.functions.iter_mut().next().unwrap();
            f.expressions[handles[0]] = naga::Expression::Relational { fun, argument: arg };
        }
        let found = check_fragment_module(&m, source).unwrap_or_else(|e| panic!("case {i}: {e}"));
        assert!(
            found.iter().any(|f| f.rule == Rule::IsInfNan),
            "IR {fun:?} did not fire: {found:?}"
        );
    }
}

#[test]
fn qa_lint_wgsl_unset_naga_is_inf_is_nan_fire() {
    naga_relational(true);
}

negative_control!(
    qa_lint_wgsl_unset_naga_is_inf_is_nan_fire,
    "the unrewritten `all` is no IsInf/IsNan",
    expected = "did not fire",
    naga_relational(false)
);

// ---------------------------------------------------------------------------------------------------------------
// A float compared with itself (R-352).

const SELF_FIRES: [&str; 13] = [
    // An argument, each of the six operators.
    "fn g(x: f32) -> bool {\n    return x != x; // fires\n}\n",
    "fn g(x: f32) -> bool {\n    return x == x; // fires\n}\n",
    "fn g(x: f32) -> bool {\n    return x < x; // fires\n}\n",
    "fn g(x: f32) -> bool {\n    return x <= x; // fires\n}\n",
    "fn g(x: f32) -> bool {\n    return x > x; // fires\n}\n",
    "fn g(x: f32) -> bool {\n    return x >= x; // fires\n}\n",
    // A private global read twice.
    "var<private> gv: f32;\nfn g() -> bool {\n    return gv != gv; // fires\n}\n",
    // A vec4 swizzle with itself.
    "fn g(v: vec4<f32>) -> vec2<bool> {\n    return v.xy != v.xy; // fires\n}\n",
    // A uniform struct member, read twice.
    "struct U { a: vec4<f32>, b: f32 }\n@group(2) @binding(0) var<uniform> u: U;\n\
     fn g() -> bool {\n    return u.a.z == u.a.z; // fires\n}\n",
    // A buffer element read twice through the same let index.
    "struct S { v: f32 }\n@group(2) @binding(0) var<storage, read> buf: array<S>;\n\
     fn g(k: u32) -> bool {\n    let i = k * 2u;\n    return buf[i].v != buf[i].v; // fires\n}\n",
    // A structurally equal expression over the same argument.
    "fn g(x: f32) -> bool {\n    return (x + 1.0) != (x + 1.0); // fires\n}\n",
    // A variable read twice, a store to another variable between.
    "fn g(x: f32) -> bool {\n    var a = x;\n    var b = x;\n    let p = a;\n    b = 2.0;\n    return p != a || b > 0.0; // fires\n}\n",
    // An f16 scalar, in a hand-written f16 file.
    "enable f16;\nfn g(h: f16) -> bool {\n    return h != h; // fires\n}\n",
];

/// The near misses: different values, different elements, reads with a store between (directly, in a nested branch,
/// through a pointer argument, to a buffer element, in a loop), and integer self-comparisons (not floats).
const SELF_QUIET: [&str; 9] = [
    "fn g(v: vec4<f32>) -> vec2<bool> {\n    return v.xy != v.yx; // fires\n}\n",
    "struct T { a: f32, b: f32 }\nfn g(s: T) -> bool {\n    return s.a == s.b; // fires\n}\n",
    "struct S { v: f32 }\n@group(2) @binding(0) var<storage, read> buf: array<S>;\n\
     fn g(i: u32) -> bool {\n    return buf[i].v == buf[i + 1u].v; // fires\n}\n",
    "fn g(x: f32, c: bool) -> bool {\n    var v = x;\n    let old = v;\n    if c { v = 0.0; }\n    return old != v; // fires\n}\n",
    "fn bump(p: ptr<function, f32>) { *p = *p + 1.0; }\n\
     fn g(x: f32) -> bool {\n    var v = x;\n    let old = v;\n    bump(&v);\n    return old != v; // fires\n}\n",
    "struct S { v: f32 }\n@group(2) @binding(0) var<storage, read_write> buf: array<S>;\n\
     fn g(i: u32) -> bool {\n    let a = buf[i].v;\n    buf[i].v = a + 1.0;\n    return a != buf[i].v; // fires\n}\n",
    "fn g(x: f32) -> f32 {\n    var v = x;\n    loop {\n        let old = v;\n        v = v * 0.5;\n        if old == v { break; } // fires\n    }\n    return v;\n}\n",
    "fn g(i: i32) -> bool {\n    return i != i; // fires\n}\n",
    "fn g(w: vec2<u32>) -> vec2<bool> {\n    return w == w; // fires\n}\n",
];

#[test]
fn qa_lint_wgsl_self_compare_fires() {
    all_fire(&SELF_FIRES, Rule::SelfCompare);
}

negative_control!(
    qa_lint_wgsl_self_compare_fires,
    "different values, or reads with a store between, are no self-comparison",
    expected = "did not fire",
    all_fire(&SELF_QUIET, Rule::SelfCompare)
);

#[test]
fn qa_lint_wgsl_self_compare_near_misses_are_quiet() {
    all_quiet(&SELF_QUIET);
}

negative_control!(
    qa_lint_wgsl_self_compare_near_misses_are_quiet,
    "self-comparisons are float-rule findings",
    expected = "float rule finding(s)",
    all_quiet(&SELF_FIRES)
);

// ---------------------------------------------------------------------------------------------------------------
// Finite-max stand-ins (R-352).

const FINITE_MAX_FIRES: [&str; 18] = [
    "fn g(d: f32) -> bool {\n    return d >= 65504.0; // fires\n}\n",
    "fn g(d: f32) -> bool {\n    return 3.4028235e38 <= d; // fires\n}\n",
    "fn g(d: f32) -> bool {\n    return d != -3.40282347e38; // fires\n}\n",
    // Negative bit patterns.
    "fn g(d: f32) -> bool {\n    return d == bitcast<f32>(0xc77fe000u); // fires\n}\n",
    "fn g(d: f32) -> bool {\n    return d < bitcast<f32>(0xff7fffffu); // fires\n}\n",
    // Conversions of integer constants.
    "fn g(d: f32) -> bool {\n    return d > f32(65504u); // fires\n}\n",
    "fn g(d: f32) -> bool {\n    return d > f32(-65504i); // fires\n}\n",
    // Arithmetic constant expressions that evaluate to it.
    "fn g(d: f32) -> bool {\n    return d > 2.0 * 32752.0; // fires\n}\n",
    "fn g(d: f32) -> bool {\n    return d > 65503.0 + 1.0; // fires\n}\n",
    "fn g(d: f32) -> bool {\n    return d > -(-65504.0); // fires\n}\n",
    "const A = 65504.0;\nconst B = A;\nfn g(d: f32) -> bool {\n    return d > B; // fires\n}\n",
    // A vector against a splat, and a typed vector constant.
    "fn g(d: vec2<f32>) -> vec2<bool> {\n    return d > vec2<f32>(65504.0); // fires\n}\n",
    "const M = vec3<f32>(65504.0, 65504.0, 65504.0);\nfn g(d: vec3<f32>) -> vec3<bool> {\n    return M < d; // fires\n}\n",
    // A negated f16 literal in a hand-written f16 file.
    "enable f16;\nfn g(h: f16) -> bool {\n    return h <= -65504h; // fires\n}\n",
    // In a fragment entry point.
    "@fragment fn fs(@location(0) d: f32) -> @location(0) vec4<f32> {\n    if d > 65504.0 { discard; } // fires\n    return vec4<f32>(d);\n}\n",
    // Constant expressions over a bitcast of a stand-in's bits that evaluate to the stand-in: scaled by one, the
    // absolute value of the negative one, and a component of a vector built from it.
    "fn g(d: f32) -> bool {\n    return d > bitcast<f32>(0x477fe000u) * 1.0; // fires\n}\n",
    "fn g(d: f32) -> bool {\n    return d > abs(bitcast<f32>(0xc77fe000u)); // fires\n}\n",
    "fn g(d: f32) -> bool {\n    return d > vec2<f32>(bitcast<f32>(0x7f7fffffu)).x; // fires\n}\n",
];

/// The near misses: values next to the stand-ins, a stand-in in arithmetic rather than as an operand, integer
/// comparisons against the stand-ins' bit patterns (bit tests, the fix), and integer 65504.
const FINITE_MAX_QUIET: [&str; 8] = [
    "fn g(d: f32) -> bool {\n    return d > 65504.00390625; // fires\n}\n",
    "fn g(d: f32) -> bool {\n    return d > -65503.0; // fires\n}\n",
    "fn g(d: f32) -> bool {\n    return d > 65505.0; // fires\n}\n",
    "fn g(d: f32) -> bool {\n    return d * 65504.0 > 1.0; // fires\n}\n",
    "fn g(d: f32) -> bool {\n    return bitcast<u32>(d) == 0x7f7fffffu; // fires\n}\n",
    "fn g(d: f32) -> bool {\n    return bitcast<u32>(d) == 0x477fe000u; // fires\n}\n",
    "fn g(i: i32) -> bool {\n    return i < 65504; // fires\n}\n",
    "fn g(w: u32) -> bool {\n    return w >= 65504u; // fires\n}\n",
];

#[test]
fn qa_lint_wgsl_finite_max_fires() {
    all_fire(&FINITE_MAX_FIRES, Rule::FiniteMax);
}

negative_control!(
    qa_lint_wgsl_finite_max_fires,
    "neighbours of the stand-ins and integer bit tests are no finite-max comparison",
    expected = "did not fire",
    all_fire(&FINITE_MAX_QUIET, Rule::FiniteMax)
);

#[test]
fn qa_lint_wgsl_finite_max_near_misses_are_quiet() {
    all_quiet(&FINITE_MAX_QUIET);
}

negative_control!(
    qa_lint_wgsl_finite_max_near_misses_are_quiet,
    "comparisons against finite-max stand-ins are float-rule findings",
    expected = "float rule finding(s)",
    all_quiet(&FINITE_MAX_FIRES)
);

// ---------------------------------------------------------------------------------------------------------------
// The workspace: every file under crates/render/frag/, the generated one included, and the real tree.

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn real_generated() -> String {
    std::fs::read_to_string(workspace_root().join(GENERATED)).expect("the generated WGSL")
}

/// A scratch workspace: a `Cargo.toml`, `generated` as the generated file, and `extra` (a path under `FRAG_DIR`, its
/// source).
fn scratch(generated: &str, extra: &[(&str, &str)]) -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "qa_m0_50_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    let write = |rel: &str, text: &str| {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    };
    write("Cargo.toml", "");
    write(GENERATED, generated);
    for (rel, text) in extra {
        write(&format!("{FRAG_DIR}/{rel}"), text);
    }
    root
}

/// `cargo xtask lint wgsl`'s `run` over a scratch workspace fails naming `file` (relative to the root), and `lint`
/// reports a finding of `rule` in it at `line`.
fn run_fails_naming(generated: &str, extra: &[(&str, &str)], file: &str, rule: Rule, line: u32) {
    let root = scratch(generated, extra);
    let result = run(&root.join("Cargo.toml"));
    let reports = lint(&root);
    let _ = std::fs::remove_dir_all(&root);
    let reports = reports.expect("the files parse and validate");
    let err = result.expect_err("lint wgsl passed");
    assert!(
        err.contains(file),
        "the failure does not name {file}: {err}"
    );
    let report = reports
        .iter()
        .find(|r| r.file == file)
        .expect("the file was linted");
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.rule == rule && f.line == Some(line)),
        "no [{rule}] at {file}:{line}: {:?}",
        report.findings
    );
}

const APPENDED_SELF: &str = "\nfn qa_nan_check(x: f32) -> bool {\n    return x != x;\n}\n";

fn generated_with_self_compare() -> (String, u32) {
    let g = real_generated();
    let line = u32::try_from(g.lines().count() + 3).unwrap();
    (format!("{g}{APPENDED_SELF}"), line)
}

#[test]
fn qa_lint_wgsl_self_compare_in_the_generated_file_fails() {
    let (g, line) = generated_with_self_compare();
    run_fails_naming(&g, &[], GENERATED, Rule::SelfCompare, line);
}

negative_control!(
    qa_lint_wgsl_self_compare_in_the_generated_file_fails,
    "the real generated file passes, so run does not fail",
    expected = "lint wgsl passed",
    run_fails_naming(&real_generated(), &[], GENERATED, Rule::SelfCompare, 1)
);

const DEEP_FILE: &str = "crates/render/frag/a/b/deep.wgsl";
const DEEP: &str = "fn g(d: f32) -> bool {\n    return d > 65504.0;\n}\n";

#[test]
fn qa_lint_wgsl_finite_max_in_a_nested_hand_written_file_fails() {
    run_fails_naming(
        &real_generated(),
        &[("a/b/deep.wgsl", DEEP)],
        DEEP_FILE,
        Rule::FiniteMax,
        2,
    );
}

negative_control!(
    qa_lint_wgsl_finite_max_in_a_nested_hand_written_file_fails,
    "the nested file with 65503.0 passes",
    expected = "lint wgsl passed",
    run_fails_naming(
        &real_generated(),
        &[("a/b/deep.wgsl", &DEEP.replace("65504.0", "65503.0"))],
        DEEP_FILE,
        Rule::FiniteMax,
        2
    )
);

/// Every finding in a multi-violation file lands on its own line: one per rule, on lines 3, 6, 9 and 12.
const MULTI: &str =
    "fn isinf(x: f32) -> bool { return (bitcast<u32>(x) & 0x7fffffffu) == 0x7f800000u; }\n\
fn a(x: f32) -> bool {\n\
    return isinf(x);\n\
}\n\
fn b(x: f32) -> bool {\n\
    return x == bitcast<f32>(0x7f800000u);\n\
}\n\
fn c(x: f32) -> bool {\n\
    return x != x;\n\
}\n\
fn d(x: f32) -> bool {\n\
    return x > 65504.0;\n\
}\n";

fn multi_lines(src: &str) {
    let found = check_fragment(src).expect("parses");
    for (rule, line) in [
        (Rule::IsInfNan, 3),
        (Rule::InfNanConstant, 6),
        (Rule::SelfCompare, 9),
        (Rule::FiniteMax, 12),
    ] {
        assert!(
            found.iter().any(|f| f.rule == rule && f.line == Some(line)),
            "[{rule}] not reported at line {line}: {found:?}"
        );
    }
}

#[test]
fn qa_lint_wgsl_each_rule_names_its_line() {
    multi_lines(MULTI);
}

negative_control!(
    qa_lint_wgsl_each_rule_names_its_line,
    "the file shifted down a line reports each finding one line later",
    expected = "not reported at line",
    multi_lines(&format!("\n{MULTI}"))
);

/// The real tree: `lint wgsl` on the workspace passes, and the generated file's `pa_d_min_is_unset` is there and
/// tests bits (the generated file is checked with every rule by `check`).
fn real_tree_clean(generated: &str) {
    assert!(
        generated.contains("fn pa_d_min_is_unset") && generated.contains("PA_D_MIN_UNSET"),
        "the generated file has no bit-pattern pa_d_min_is_unset"
    );
    let found = check(generated).expect("the generated WGSL parses and validates");
    assert!(
        found.is_empty(),
        "findings on the generated file: {found:?}"
    );
}

#[test]
fn qa_lint_wgsl_workspace_passes() {
    run(&workspace_root().join("Cargo.toml")).expect("lint wgsl on the workspace");
    real_tree_clean(&real_generated());
}

negative_control!(
    qa_lint_wgsl_workspace_passes,
    "pa_d_min_is_unset rewritten as an isinf-style self test fails",
    expected = "findings on the generated file",
    real_tree_clean(&format!(
        "{}\nfn qa_unset(d: f32) -> bool {{ return d != d; }}\n",
        real_generated()
    ))
);
