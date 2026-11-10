//! qa's tests for TASK-M7-54's line-break fix (REQ-RENDER-103: the generated WGSL of a graph compiles under naga and
//! holds each node's passes), written from WGSL's rule, not from the implementation: a `//` comment ends at a line
//! break (WGSL, "Comments"; "Blankspace and line breaks"), and the line breaks are the line feed, vertical tab, form
//! feed, carriage return, U+0085, U+2028 and U+2029; the space, the tab and the no-break space U+00A0 are none.
//!
//! - Module-level helpers (a `fn`, a `const`) each declared just after a `//` comment that ends at a line break other
//!   than the line feed, one comment holding a `/*` that opens nothing, stay code: two nodes of the pass generate a
//!   module that validates under naga, so each node's helpers are its own, and the module holds each node's pass with
//!   its line breaks as written.
//!
//! Each test registers its negative control (R-176).

use naga::valid::{Capabilities, ValidationFlags, Validator};
use render::print::codegen::generate;
use render::print::graph::{expand, Graph};
use render::print::registry::Registry;
use render::print::shipped;
use validation::negative_control;

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

/// Two `qa_brk` nodes in a chain, the Render node feeding the first and the output reading the second.
const TWO: &str = r#"{"format":"principia.print/1","name":"Two qa_brk","nodes":{"img":{"type":"render"},
  "a":{"type":"qa_brk","params":{"amount":0.3},"in":{"in":"img"}},
  "b":{"type":"qa_brk","params":{"amount":0.7},"in":{"in":"a"}},
  "out":{"type":"output","in":{"in":"b"}}},"out":"out"}"#;

/// The module two `qa_brk` nodes of `pass` generate, validated under naga; every failure panics "qa breaks:".
fn two_node_module(pass: &str) -> String {
    let mut r: Registry = shipped().registry.clone();
    let text = pass.to_owned();
    let read = move |file: &str| (file == "qa_brk.wgsl").then(|| text.clone());
    if let Err(e) = r.register(&decl("qa_brk"), &read) {
        panic!("qa breaks: the pass is refused: {e}");
    }
    let e = expand(&Graph::parse(TWO).expect("parses"), &|_: &str| None).expect("expands");
    let source = match generate(&e, &r) {
        Ok(g) => g.source,
        Err(e) => panic!("qa breaks: the graph does not generate: {e}"),
    };
    let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|e| {
        panic!(
            "qa breaks: the generated WGSL does not parse: {}",
            e.emit_to_string(&source)
        )
    });
    if let Err(e) =
        Validator::new(ValidationFlags::all(), Capabilities::default()).validate(&module)
    {
        panic!(
            "qa breaks: the generated WGSL does not validate: {}",
            e.emit_to_string(&source)
        );
    }
    source
}

/// WGSL's line breaks other than the line feed.
const BREAKS: [char; 6] = ['\u{B}', '\u{C}', '\r', '\u{85}', '\u{2028}', '\u{2029}'];

/// A pass whose helpers each follow a `//` comment ended by `brk`, one comment holding a `/*` that opens nothing.
fn pass_with(brk: char) -> String {
    format!(
        "// the weight /* not a block comment{brk}const qa_k: f32 = 0.5;\n\
         // the helper{brk}fn qa_h(c: vec4<f32>) -> vec4<f32> {{ return c * qa_k; }}\n\
         fn run(uv: vec2<f32>) -> vec4<f32> {{ return qa_h(tex(in_in, uv)) * p_amount; }}\n"
    )
}

/// For each line break, `make(brk)` generates and validates as a two-node module holding each node's pass, each
/// comment and the line break after it as written.
fn check_breaks(make: fn(char) -> String) {
    for brk in BREAKS {
        let source = two_node_module(&make(brk));
        for comment in [
            format!("// the weight /* not a block comment{brk}"),
            format!("// the helper{brk}"),
        ] {
            assert_eq!(
                source.matches(comment.as_str()).count(),
                2,
                "{brk:?}: the module does not hold each node's pass, `{comment:?}` as written"
            );
        }
    }
}

#[test]
fn qa_print_helpers_after_a_line_comment_ended_by_each_wgsl_line_break() {
    check_breaks(pass_with);
}

negative_control!(
    qa_print_helpers_after_a_line_comment_ended_by_each_wgsl_line_break,
    "each line break replaced by a no-break space U+00A0, which ends no WGSL comment: the helpers are commented out",
    expected = "qa breaks:",
    check_breaks(|_| pass_with('\u{A0}'))
);
