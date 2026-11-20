//! qa's tests for TASK-M7-54's second fix round (REQ-RENDER-103: the generated WGSL of a graph compiles under naga and
//! holds each node's passes), written from the requirement's statement, not from the implementation.
//!
//! A pass's comments are text, never code: a comment ending in `.` does not make the name after it a member, so a use
//! of the pass's own helper written straight after such a comment is that helper; and a comment between a `.` and a
//! name leaves the name a member. Two nodes of one type, its pass declaring a module-level `var`, `const` and `struct`
//! and using them so, generate a module that validates under naga.
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

fn decl(id: &str, passes: &[&str]) -> String {
    serde_json::json!({
        "id": id,
        "label": format!("QA {id}"),
        "category": "op",
        "doc": "A qa test entry.",
        "inputs": [{ "name": "in", "type": "image", "default": "render" }],
        "outputs": [{ "name": "out", "type": "image" }],
        "params": [{ "name": "amount", "kind": "number", "default": 0.5, "min": 0, "max": 1, "step": 0.01, "unit": "fraction" }],
        "passes": passes,
        "cost": "light",
        "motion": "full",
        "temporally_stable": true,
        "screen_medium": false,
        "deviations": ["A qa test entry."]
    })
    .to_string()
}

/// A pass whose own names are used straight after comments ending in `.`, and whose struct's member, named as its own
/// `const`, is read across a comment after the `.`.
const COMMENTED: &str = "var<private> qa_acc: f32;
const qa_k: f32 = 0.5;
struct QaQ { qa_k: f32 }
fn run(uv: vec2<f32>) -> vec4<f32> {
    let q = QaQ(0.25);
    // Accumulate.
    qa_acc = qa_k;
    /* acc. */ qa_acc = qa_acc + q. /* member */ qa_k;
    return tex(in_in, uv) * qa_acc * p_amount;
}
";

fn wgsl(file: &str) -> Option<String> {
    (file == "qa_commented.wgsl").then(|| COMMENTED.to_owned())
}

const TWO: &str = r#"{"format":"principia.print/1","name":"Two commented","nodes":{"img":{"type":"render"},
  "a":{"type":"qa_commented","params":{"amount":0.3},"in":{"in":"img"}},
  "b":{"type":"qa_commented","params":{"amount":0.7},"in":{"in":"a"}},
  "out":{"type":"output","in":{"in":"b"}}},"out":"out"}"#;

/// The module of two `qa_commented` nodes, as `emit` hands it on, validates under naga and holds both nodes' passes.
fn check(emit: fn(String) -> String) {
    let mut r: Registry = shipped().registry.clone();
    r.register(&decl("qa_commented", &["qa_commented.wgsl"]), &wgsl)
        .expect("qa_commented registers");
    let e = expand(
        &Graph::parse(TWO).unwrap_or_else(|e| panic!("parse: {e}")),
        &|_: &str| None,
    )
    .expect("expands");
    let g = generate(&e, &r).unwrap_or_else(|e| panic!("does not generate: {e}"));
    let source = emit(g.source);
    compile(&source);
    for comment in ["// Accumulate.", "/* acc. */", "/* member */"] {
        assert_eq!(
            source.matches(comment).count(),
            2,
            "the module does not hold each node's pass, comment `{comment}`"
        );
    }
}

/// `source` with the identifier after each `marker` (and its spaces) rewritten by `f`.
#[cfg(feature = "controls")]
fn after(source: &str, marker: &str, f: fn(&str) -> String) -> String {
    let mut out = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(i) = rest.find(marker) {
        let j = i + marker.len();
        out.push_str(&rest[..j]);
        rest = &rest[j..];
        let ws = rest.len() - rest.trim_start().len();
        out.push_str(&rest[..ws]);
        rest = &rest[ws..];
        let end = rest
            .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
            .unwrap_or(rest.len());
        out.push_str(&f(&rest[..end]));
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

/// The name as the pass wrote it, whatever namespace the module gave it.
#[cfg(feature = "controls")]
fn bare(word: &str) -> String {
    word.rsplit_once("_h_").map_or(word, |(_, w)| w).to_owned()
}

/// The name namespaced as though it were the pass's own helper.
#[cfg(feature = "controls")]
fn namespaced(word: &str) -> String {
    format!("qa_ns_h_{word}")
}

#[test]
fn qa_print_use_after_a_comment_ending_in_a_dot_is_the_helper() {
    check(|s| s);
}

negative_control!(
    qa_print_use_after_a_comment_ending_in_a_dot_is_the_helper,
    "each use after a comment ending in `.` left as the pass wrote it",
    expected = "the generated WGSL does not",
    check(|s| after(&after(&s, "// Accumulate.", bare), "/* acc. */", bare))
);

#[test]
fn qa_print_member_after_a_dot_and_a_comment_stays_a_member() {
    check(|s| s);
}

negative_control!(
    qa_print_member_after_a_dot_and_a_comment_stays_a_member,
    "the member after a `.` and a comment namespaced as a helper",
    expected = "the generated WGSL does not",
    check(|s| after(&s, "/* member */", namespaced))
);
