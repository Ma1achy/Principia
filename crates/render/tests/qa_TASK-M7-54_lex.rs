//! qa's tests for TASK-M7-54's comment handling (REQ-RENDER-103: every registered entry carries its passes, and the
//! generated WGSL of a graph compiles under naga and holds each node's passes), written from the requirement and
//! WGSL's own comment rule, not from the implementation: WGSL's block comments nest, so `/* a /* b */ c */` is one
//! comment, and what follows the inner `*/` is still inside it.
//!
//! - A pass whose only `fn run` sits after the inner `*/` of a nested comment defines no `run`, and is refused.
//! - A member read after a `.` and a nested comment (holding non-ASCII text) stays a member, so two nodes of the
//!   pass generate a module that validates under naga and holds both passes.
//! - A pass whose nested block comment is never closed is refused before any module is handed on.
//!
//! WGSL has no string literals, so no case here puts a comment marker inside a string.
//!
//! Each test registers its negative control (R-176).

use naga::valid::{Capabilities, ValidationFlags, Validator};
use render::print::codegen::generate;
use render::print::graph::{expand, Graph};
use render::print::registry::Registry;
use render::print::shipped;
use validation::negative_control;

fn compile(source: &str) {
    let module = naga::front::wgsl::parse_str(source).unwrap_or_else(|e| {
        panic!(
            "the generated WGSL does not parse: {}",
            e.emit_to_string(source)
        )
    });
    Validator::new(ValidationFlags::all(), Capabilities::default())
        .validate(&module)
        .unwrap_or_else(|e| {
            panic!(
                "the generated WGSL does not validate: {}",
                e.emit_to_string(source)
            )
        });
}

fn decl(id: &str) -> String {
    serde_json::json!({
        "id": id,
        "label": format!("QA {id}"),
        "category": "op",
        "doc": "A qa test entry.",
        "inputs": [{ "name": "in", "type": "image", "default": "render" }],
        "outputs": [{ "name": "out", "type": "image" }],
        "params": [{ "name": "amount", "kind": "number", "default": 0.5, "min": 0, "max": 1, "step": 0.01, "unit": "fraction" }],
        "passes": [format!("{id}.wgsl")],
        "cost": "light",
        "motion": "full",
        "temporally_stable": true,
        "screen_medium": false,
        "deviations": ["A qa test entry."]
    })
    .to_string()
}

/// The shipped registry with `qa_lex` registered, its one pass `pass`.
fn registry(pass: &str) -> Result<Registry, String> {
    let mut r: Registry = shipped().registry.clone();
    let text = pass.to_owned();
    let read = move |file: &str| (file == "qa_lex.wgsl").then(|| text.clone());
    r.register(&decl("qa_lex"), &read)
        .map(|_| ())
        .map_err(|e| format!("{e}"))?;
    Ok(r)
}

/// Two `qa_lex` nodes in a chain, the Render node feeding the first and the output reading the second.
const TWO: &str = r#"{"format":"principia.print/1","name":"Two qa_lex","nodes":{"img":{"type":"render"},
  "a":{"type":"qa_lex","params":{"amount":0.3},"in":{"in":"img"}},
  "b":{"type":"qa_lex","params":{"amount":0.7},"in":{"in":"a"}},
  "out":{"type":"output","in":{"in":"b"}}},"out":"out"}"#;

/// The module of `TWO` over `r`, or the codegen's refusal.
fn module(r: &Registry) -> Result<String, String> {
    let e = expand(
        &Graph::parse(TWO).unwrap_or_else(|e| panic!("parse: {e}")),
        &|_: &str| None,
    )
    .expect("expands");
    generate(&e, r)
        .map(|g| g.source)
        .map_err(|e| format!("{e}"))
}

// ----- `fn run` after a nested comment's inner `*/` -----

const RUN_IN_NESTED: &str =
    "/* outer /* inner */ fn run(uv: vec2<f32>) -> vec4<f32> { return vec4<f32>(uv, 0.0, 1.0); } */
fn runner(uv: vec2<f32>) -> vec4<f32> { return vec4<f32>(uv, 0.0, 1.0); }
";

/// `pass` is refused as defining no `fn run`; and the same nested comment above a real `fn run` registers, generates
/// and validates, the commented `run` never the module's.
fn check_run_in_nested(pass: &str) {
    match registry(pass) {
        Ok(_) => panic!("a pass whose only `fn run` is inside a nested comment registered"),
        Err(e) => assert!(
            e.contains("fn run"),
            "the refusal does not name `fn run`: {e}"
        ),
    }
    let real = format!(
        "{RUN_IN_NESTED}fn run(uv: vec2<f32>) -> vec4<f32> {{ return runner(uv) * p_amount; }}\n"
    );
    let r =
        registry(&real).unwrap_or_else(|e| panic!("the pass with a real `fn run` is refused: {e}"));
    let source = module(&r).unwrap_or_else(|e| panic!("does not generate: {e}"));
    compile(&source);
    assert_eq!(
        source.matches("/* outer /* inner */ fn run(").count(),
        2,
        "the module does not hold each node's pass, its nested comment as written"
    );
}

#[test]
fn qa_print_run_after_a_nested_comments_inner_close_is_no_run() {
    check_run_in_nested(RUN_IN_NESTED);
}

negative_control!(
    qa_print_run_after_a_nested_comments_inner_close_is_no_run,
    "the pass as a lexer ending a block comment at its first `*/` reads it: `fn run` outside the comment",
    expected = "inside a nested comment registered",
    check_run_in_nested(&RUN_IN_NESTED.replacen("/* outer /* inner */", "/* outer inner */", 1))
);

// ----- a member after a `.` and a nested comment -----

/// A struct member named as the pass's own `const`, read after a `.` and a nested comment holding non-ASCII text, and
/// the `const` itself read beside it.
const MEMBER: &str = "struct QaS { qa_m: f32 }
const qa_m: f32 = 0.5;
fn run(uv: vec2<f32>) -> vec4<f32> {
    let s = QaS(0.25);
    /* é /* ∂ */ ü */
    return tex(in_in, uv) * (s. /* a /* b */ c */ qa_m + qa_m) * p_amount;
}
";

fn check_member(emit: fn(String) -> String) {
    let r = registry(MEMBER).unwrap_or_else(|e| panic!("qa_lex is refused: {e}"));
    let source = emit(module(&r).unwrap_or_else(|e| panic!("does not generate: {e}")));
    compile(&source);
    for comment in ["/* é /* ∂ */ ü */", "s. /* a /* b */ c */ qa_m"] {
        assert_eq!(
            source.matches(comment).count(),
            2,
            "the module does not hold each node's pass, `{comment}` as written"
        );
    }
}

#[test]
fn qa_print_member_after_a_nested_comment_stays_a_member() {
    check_member(|s| s);
}

negative_control!(
    qa_print_member_after_a_nested_comment_stays_a_member,
    "the member renamed as a helper, as a lexer ending the comment at its first `*/` (`c` then code) leaves it",
    expected = "the generated WGSL does not",
    check_member(|s| s.replace("c */ qa_m", "c */ qa_ns_h_qa_m"))
);

// ----- a nested block comment never closed -----

const UNCLOSED: &str = "fn run(uv: vec2<f32>) -> vec4<f32> { return tex(in_in, uv) * p_amount; }
/* open /* nested */ still open
";

/// `pass`, its nested comment never closed, is refused, at registration or by the codegen, before a module is
/// handed on.
fn check_unclosed(pass: &str) {
    let refusal = match registry(pass) {
        Err(e) => e,
        Ok(r) => match module(&r) {
            Ok(source) => panic!("a pass with an unclosed nested comment generated:\n{source}"),
            Err(e) => e,
        },
    };
    assert!(
        refusal.contains("/*") || refusal.contains("unclosed"),
        "the refusal does not name the unclosed comment: {refusal}"
    );
}

#[test]
fn qa_print_unclosed_nested_comment_is_refused() {
    check_unclosed(UNCLOSED);
}

negative_control!(
    qa_print_unclosed_nested_comment_is_refused,
    "the pass's nested comment closed, as a lexer ending it at its first `*/` reads it",
    expected = "with an unclosed nested comment generated",
    check_unclosed(&UNCLOSED.replace("still open\n", "still open */\n"))
);
