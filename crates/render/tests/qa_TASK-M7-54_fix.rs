//! qa's tests for TASK-M7-54's fix round (REQ-RENDER-101, REQ-RENDER-103), written from the requirements' statements
//! and verifies and from D-15, as the owner let it stand ("A preset node takes one input, and its unconnected inputs
//! read it"), not from the implementation.
//!
//! - REQ-RENDER-101 (a preset usable as a node, running its graph as a sub-graph): a preset whose output node is left
//!   unconnected passes the picture coming in through, end to end into the generated module's texture reads: the
//!   preset node's input where one is wired, the render where none is, and, nested in another preset, the outer preset
//!   node's input; and such a preset passes the build-time check beside the shipped presets.
//! - REQ-RENDER-103 (the generated WGSL of a graph compiles under naga and holds each node's passes): two nodes of one
//!   type whose passes declare module-level helpers (a `fn` reading the node's input and parameter, a `const`, a
//!   `struct`), its two passes declaring a helper of one name, generate a module that validates under naga, holds
//!   each node's passes, and has one fragment entry point per pass.
//!
//! Each test registers its negative control (R-176).

use naga::valid::{Capabilities, ValidationFlags, Validator};
use render::print::codegen::{generate, Generated, Reads};
use render::print::graph::{expand, Graph};
use render::print::registry::{check, Registry};
use render::print::{shipped, NODES, PRESETS, WGSL};
use validation::negative_control;

fn compile(source: &str) -> naga::Module {
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
    module
}

/// A §4.3 declaration, every field present, of one image input defaulting to the render and an `amount` fraction.
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

/// A pass as a lab port writes one: helpers at module level, one reading the node's input and parameter.
const HELPED_A: &str = "const QA_GAIN: f32 = 0.5;
struct QaPair { a: f32, b: f32 }
fn qa_luma(c: vec3<f32>) -> f32 { return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722)); }
fn qa_sample(uv: vec2<f32>) -> vec4<f32> { return tex(in_in, uv) * p_amount; }
fn run(uv: vec2<f32>) -> vec4<f32> {
    let c = qa_sample(uv);
    let p = QaPair(qa_luma(c.rgb), QA_GAIN);
    return vec4<f32>(c.rgb * p.a * p.b, 1.0);
}
";

/// Its second pass, declaring a helper of the first pass's name.
const HELPED_B: &str = "fn qa_luma(c: vec3<f32>) -> f32 { return max(c.r, max(c.g, c.b)); }
fn run(uv: vec2<f32>) -> vec4<f32> {
    return vec4<f32>(vec3<f32>(qa_luma(tex(prev, uv).rgb)), 1.0);
}
";

/// A one-pass tint, no helpers.
const TINT: &str = "fn run(uv: vec2<f32>) -> vec4<f32> {
    return tex(in_in, uv) * vec4<f32>(0.25, 0.5, 0.75, 1.0) * p_amount;
}
";

fn wgsl(file: &str) -> Option<String> {
    match file {
        "qa_helped_a.wgsl" => Some(HELPED_A.into()),
        "qa_helped_b.wgsl" => Some(HELPED_B.into()),
        "qa_tint.wgsl" => Some(TINT.into()),
        _ => None,
    }
}

fn registry() -> Registry {
    let mut r = shipped().registry.clone();
    r.register(
        &decl("qa_helped", &["qa_helped_a.wgsl", "qa_helped_b.wgsl"]),
        &wgsl,
    )
    .expect("qa_helped registers");
    r.register(&decl("qa_tint", &["qa_tint.wgsl"]), &wgsl)
        .expect("qa_tint registers");
    r
}

fn parse(text: &str) -> Graph {
    Graph::parse(text).unwrap_or_else(|e| panic!("parse: {e}"))
}

// ───────────────────────────── REQ-RENDER-103: two nodes of a helper-bearing type ─────────────────────────────

const TWO_HELPED: &str = r#"{"format":"principia.print/1","name":"Two helped","nodes":{"img":{"type":"render"},
  "a":{"type":"qa_helped","params":{"amount":0.3},"in":{"in":"img"}},
  "b":{"type":"qa_helped","params":{"amount":0.7},"in":{"in":"a"}},
  "out":{"type":"output","in":{"in":"b"}}},"out":"out"}"#;

#[cfg(feature = "controls")]
const HELPERS: [&str; 4] = ["qa_luma", "qa_sample", "QA_GAIN", "QaPair"];

/// The module, as `emit` hands it on, validates under naga, holds each node's two passes, and has one fragment entry
/// point per pass of its nodes (render none, two helped two each, output one).
fn check_two_helped(emit: fn(String) -> String) {
    let r = registry();
    let e = expand(&parse(TWO_HELPED), &|_: &str| None).expect("expands");
    let g = generate(&e, &r).unwrap_or_else(|e| panic!("does not generate: {e}"));
    let source = emit(g.source);
    let module = compile(&source);
    for body in [
        "vec3<f32>(0.2126, 0.7152, 0.0722)",
        "max(c.r, max(c.g, c.b))",
    ] {
        assert_eq!(
            source.matches(body).count(),
            2,
            "the module does not hold each node's pass `{body}`"
        );
    }
    let fragments = module
        .entry_points
        .iter()
        .filter(|ep| ep.stage == naga::ShaderStage::Fragment)
        .count();
    assert_eq!(fragments, 5, "not one fragment entry point per pass");
}

/// Every identifier ending `_<helper>` written back as the bare helper, whatever namespace the module gave it: each
/// node's helpers declared under the names the pass wrote.
#[cfg(feature = "controls")]
fn bare_helpers(source: String) -> String {
    let mut out = String::with_capacity(source.len());
    let mut rest = source.as_str();
    while let Some(c) = rest.chars().next() {
        if c.is_ascii_alphabetic() || c == '_' {
            let end = rest
                .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
                .unwrap_or(rest.len());
            let word = &rest[..end];
            let bare = HELPERS
                .iter()
                .find(|h| word.len() > h.len() && word.ends_with(&format!("_{h}")));
            out.push_str(bare.copied().unwrap_or(word));
            rest = &rest[end..];
        } else {
            out.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    out
}

#[test]
fn qa_print_two_nodes_of_a_helper_bearing_type_compile() {
    check_two_helped(|s| s);
}

negative_control!(
    qa_print_two_nodes_of_a_helper_bearing_type_compile,
    "each node's helpers declared under the pass's own names",
    expected = "the generated WGSL does not",
    check_two_helped(bare_helpers)
);

// ───────────────────────────── REQ-RENDER-101: a preset's unconnected output (D-15) ─────────────────────────────

/// A preset whose output node is left unconnected: it passes the picture coming in through.
const QA_BARE: &str =
    r#"{"format":"principia.print/1","name":"QA Bare","nodes":{"o":{"type":"output"}},"out":"o"}"#;

/// A preset holding QA Bare, its preset node's input unconnected, and its own output unconnected.
const QA_OUTER: &str = r#"{"format":"principia.print/1","name":"QA Outer","nodes":{"i":{"type":"preset:QA Bare"},
  "o":{"type":"output"}},"out":"o"}"#;

/// A preset that changes the picture: its tint reads the picture coming in, its output the tint.
#[cfg(feature = "controls")]
const QA_TINTED: &str = r#"{"format":"principia.print/1","name":"QA Bare","nodes":{"u":{"type":"qa_tint"},
  "o":{"type":"output","in":{"in":"u"}}},"out":"o"}"#;

/// The render, tinted, through preset node `p` of type `preset`.
fn through(preset: &str) -> String {
    format!(
        r#"{{"format":"principia.print/1","name":"G","nodes":{{"img":{{"type":"render"}},
        "t":{{"type":"qa_tint","in":{{"in":"img"}}}},"p":{{"type":"preset:{preset}","in":{{"in":"t"}}}},
        "out":{{"type":"output","in":{{"in":"p"}}}}}},"out":"out"}}"#
    )
}

/// Preset node `p` of type `preset`, unwired.
const UNWIRED: &str = r#"{"format":"principia.print/1","name":"G","nodes":{"p":{"type":"preset:QA Bare"},
  "out":{"type":"output","in":{"in":"p"}}},"out":"out"}"#;

fn generated(text: &str, bare: &str) -> Generated {
    let presets = |name: &str| match name {
        "QA Bare" => Some(parse(bare)),
        "QA Outer" => Some(parse(QA_OUTER)),
        _ => None,
    };
    let e = expand(&parse(text), &presets)
        .unwrap_or_else(|e| panic!("the preset node does not expand: {e}"));
    generate(&e, &registry()).unwrap_or_else(|e| panic!("does not generate: {e}"))
}

/// The module's texture reads.
fn reads(g: &Generated) -> Vec<&Reads> {
    g.textures.iter().map(|t| &t.reads).collect()
}

/// `bare`, as QA Bare, passes the picture coming in through: wired from the tint, directly and nested in QA Outer,
/// the output reads the tint and the tint the render; unwired, the output reads the render.
fn check_pass_through(bare: &str) {
    for preset in ["QA Bare", "QA Outer"] {
        let g = generated(&through(preset), bare);
        let r = reads(&g);
        let tint = r
            .iter()
            .filter(|r| matches!(r, Reads::Node { node, .. } if node == "t"))
            .count();
        assert_eq!(
            (tint, r.len()),
            (1, 2),
            "through `{preset}`, the unconnected output does not read the picture coming in: {r:?}"
        );
        assert!(
            r.contains(&&Reads::Source),
            "the tint reads no render: {r:?}"
        );
    }
    let g = generated(UNWIRED, bare);
    let r = reads(&g);
    assert!(
        !r.is_empty() && r.iter().all(|r| **r == Reads::Source),
        "unwired, the unconnected output does not read the render: {r:?}"
    );
}

#[test]
fn qa_print_preset_unconnected_output_passes_the_picture_through() {
    check_pass_through(QA_BARE);
    // The build-time check passes it beside the shipped registry and presets.
    let presets: Vec<(&str, &str)> = PRESETS
        .iter()
        .copied()
        .chain([("QA Bare.json", QA_BARE), ("QA Outer.json", QA_OUTER)])
        .collect();
    check(NODES, WGSL, &presets).unwrap_or_else(|e| panic!("the check refuses QA Bare: {e}"));
}

negative_control!(
    qa_print_preset_unconnected_output_passes_the_picture_through,
    "a preset that tints the picture coming in",
    expected = "the unconnected output does not read the picture coming in",
    check_pass_through(QA_TINTED)
);
