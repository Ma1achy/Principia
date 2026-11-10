//! qa's tests for TASK-M7-54 (REQ-RENDER-101 to REQ-RENDER-104; R-486, R-492, R-558), written from the requirements'
//! statements and verifies and from principia_print_graph.md §4.1 to §4.3 and §11 test 8, not from the implementation.
//!
//! - REQ-RENDER-101: §4.1's Linocut graph, as the design doc prints it, serialises to the JCS text qa writes out by
//!   hand (RFC 8785: members sorted by UTF-16 code units, ECMAScript numbers, no whitespace), and that text reads back
//!   bitwise; JCS's number form (RFC 8785 §3.2.2.3 and its Appendix B examples) and its UTF-16 member order, which a
//!   byte-order sort gets wrong; graphs equal but for member order serialise equal; "$depth" and "$seed+2" read the
//!   setting and the setting plus 2 through the expansion; "$missing" fails naming the setting; `preset:Polaroid` runs
//!   Polaroid's graph as a sub-graph, its input the preset node's, nested presets too; a preset that includes itself,
//!   directly or through another, fails naming the cycle; a graph carrying editor positions, at any level, is refused.
//! - REQ-RENDER-102: every pair of the five port types, wired source to sink, is accepted exactly when the types match,
//!   and each refusal names both ports and both types; every declared default feeds an unconnected input, linocut's
//!   `ground` paper and `field` white; a sixth port type does not register.
//! - REQ-RENDER-103: an entry lacking any one of §4.3's thirteen declared fields fails to register, as does one with a
//!   cost, motion policy or flag outside its set; the shipped Render node's category reads "Print source" and no
//!   category reads "Source" or "post"; a two-node graph's generated module validates under naga, holds each node's
//!   pass and has one fragment entry point per pass; the stain's codegen refuses each of them.
//! - REQ-RENDER-104: the build-time check (`registry::check`) passes the shipped registry and presets; refuses an
//!   out-of-range value reached through a `$` setting or an offset, and one just past either end of the range, naming
//!   the preset, the node and the parameter, while the ends pass; refuses a preset naming a preset that is not there;
//!   and a number parameter without a range, a step or a unit, or with a unit outside §6.2's, fails to register.
//!
//! Each test registers its negative control (R-176).

use std::collections::BTreeMap;

use naga::valid::{Capabilities, ValidationFlags, Validator};
use render::codegen::{generate as stain_generate, Arg, Call, Node as StainNode, Slot, ValueType};
use render::print::codegen::generate;
use render::print::graph::{expand, number, resolve, Expanded, Graph, Value};
use render::print::ports::{validate, Feed, InputDefault, PortError, Wiring};
use render::print::registry::{check, Category, Registry};
use render::print::{shipped, NODES, PRESETS, WGSL};
use validation::negative_control;

#[path = "support/print_fixtures.rs"]
#[allow(dead_code)]
mod support;

// ───────────────────────────── REQ-RENDER-101: the format ─────────────────────────────

/// §4.1's example, as the design doc prints it.
const DOC_LINOCUT: &str = r##"{
  "format": "principia.print/1",
  "name": "Linocut",
  "settings": { "seed": 3, "depth": 0.7 },
  "nodes": {
    "img":  { "type": "render" },
    "fl":   { "type": "flow",    "params": { "sigma": 5 },        "in": { "in": "img" } },
    "pap":  { "type": "paper",   "params": { "colour": "#efe9dc" } },
    "cut":  { "type": "linocut", "params": { "depth": "$depth", "seed": "$seed" },
              "in": { "in": "img", "flow": "fl", "paper": "pap" } },
    "out":  { "type": "output",  "in": { "in": "cut" } }
  },
  "out": "out"
}"##;

/// The same graph in JCS, written out by qa from RFC 8785: every object's members sorted, no whitespace.
const DOC_LINOCUT_JCS: &str = concat!(
    r##"{"format":"principia.print/1","name":"Linocut","nodes":{"##,
    r##""cut":{"in":{"flow":"fl","in":"img","paper":"pap"},"params":{"depth":"$depth","seed":"$seed"},"type":"linocut"},"##,
    r##""fl":{"in":{"in":"img"},"params":{"sigma":5},"type":"flow"},"##,
    r##""img":{"type":"render"},"##,
    r##""out":{"in":{"in":"cut"},"type":"output"},"##,
    r##""pap":{"params":{"colour":"#efe9dc"},"type":"paper"}},"##,
    r##""out":"out","settings":{"depth":0.7,"seed":3}}"##
);

type Serialiser = fn(&str) -> String;

/// The crate's serialisation of the graph `text`.
fn canonical(text: &str) -> String {
    Graph::parse(text)
        .unwrap_or_else(|e| panic!("the graph does not parse: {e}"))
        .canonical()
        .unwrap_or_else(|e| panic!("the graph does not serialise: {e}"))
}

/// The graph's JCS form is qa's, and that form reads back to itself bitwise.
fn check_doc_linocut(serialise: Serialiser) {
    let once = serialise(DOC_LINOCUT);
    assert_eq!(
        once, DOC_LINOCUT_JCS,
        "§4.1's Linocut is not serialised in JCS"
    );
    let twice = serialise(&once);
    assert_eq!(
        twice.as_bytes(),
        once.as_bytes(),
        "the canonical text does not read back bitwise"
    );
}

#[test]
fn qa_print_doc_linocut_is_jcs_and_round_trips() {
    check_doc_linocut(canonical);
}

negative_control!(
    qa_print_doc_linocut_is_jcs_and_round_trips,
    "a serialiser that writes the authored text back",
    expected = "§4.1's Linocut is not serialised in JCS",
    check_doc_linocut(|t| t.to_owned())
);

/// Two node ids, and two setting names, whose UTF-16 order (RFC 8785 §3.2.3) is the reverse of their UTF-8 order:
/// U+1F600 is the surrogate pair D83D DE00, before U+E000 in UTF-16, after it in UTF-8.
const UTF16_ORDER: &str = "{\"format\":\"principia.print/1\",\"name\":\"U\",\
    \"settings\":{\"\u{e000}\":1,\"\u{1f600}\":2},\
    \"nodes\":{\"\u{e000}\":{\"type\":\"render\"},\"\u{1f600}\":{\"type\":\"output\",\"in\":{\"in\":\"\u{e000}\"}}},\
    \"out\":\"\u{1f600}\"}";

fn check_utf16_order(serialise: Serialiser) {
    let text = serialise(UTF16_ORDER);
    let at = |key: &str| {
        text.find(&format!("\"{key}\":"))
            .unwrap_or_else(|| panic!("{key:?} is not a member of {text}"))
    };
    // Node ids: the emoji node's key comes first. Settings: likewise, searched after "settings".
    assert!(
        at("\u{1f600}") < at("\u{e000}"),
        "members are not sorted by UTF-16 code units: {text}"
    );
    let settings = &text[text.find("\"settings\":").expect("settings")..];
    let s = |key: &str| settings.find(&format!("\"{key}\":")).expect("a setting");
    assert!(
        s("\u{1f600}") < s("\u{e000}"),
        "settings are not sorted by UTF-16 code units: {text}"
    );
}

#[test]
fn qa_print_jcs_sorts_by_utf16() {
    check_utf16_order(canonical);
}

negative_control!(
    qa_print_jcs_sorts_by_utf16,
    "a serialiser sorting members by their UTF-8 bytes",
    expected = "members are not sorted by UTF-16 code units",
    check_utf16_order(|t| {
        let v: serde_json::Value = serde_json::from_str(t).expect("JSON");
        // serde_json's map is a BTreeMap<String, _>: byte order.
        serde_json::to_string(&v).expect("serialises")
    })
);

/// RFC 8785 §3.2.2.3 (ECMAScript's Number::toString) and its Appendix B's examples.
const JCS_NUMBERS: [(f64, &str); 14] = [
    (0.0, "0"),
    (-0.0, "0"),
    (-1.0, "-1"),
    (5e-324, "5e-324"),
    (1.7976931348623157e308, "1.7976931348623157e+308"),
    (1e21, "1e+21"),
    (999999999999999900000.0, "999999999999999900000"),
    (1e23, "1e+23"),
    (1e-7, "1e-7"),
    (0.000001, "0.000001"),
    (333333333.3333333, "333333333.3333333"),
    (0.1 + 0.2, "0.30000000000000004"),
    (4.5, "4.5"),
    (295147905179352830000.0, "295147905179352830000"),
];

fn check_numbers(write: fn(f64) -> String) {
    for (x, want) in JCS_NUMBERS {
        assert_eq!(write(x), want, "{x:e} is not written in JCS's form");
    }
}

#[test]
fn qa_print_jcs_numbers() {
    check_numbers(|x| number(x).unwrap_or_else(|e| panic!("{x}: {e}")));
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(
            number(bad).is_err(),
            "{bad} has no JSON form, yet was written"
        );
    }
    // And through a graph: a setting of 1e21 and one of 1e-7 are written in JCS's form.
    let text = DOC_LINOCUT.replace(
        "\"seed\": 3, \"depth\": 0.7",
        "\"seed\": 1e21, \"depth\": 1e-7",
    );
    let out = canonical(&text);
    assert!(
        out.contains("\"settings\":{\"depth\":1e-7,\"seed\":1e+21}"),
        "settings not written in JCS: {out}"
    );
}

negative_control!(
    qa_print_jcs_numbers,
    "Rust's own float formatting",
    expected = "is not written in JCS's form",
    check_numbers(|x| x.to_string())
);

/// The doc's Linocut with every object's members in another order, and other whitespace.
const DOC_LINOCUT_REORDERED: &str = r##"{"out":"out","nodes":{
  "out":{"in":{"in":"cut"},"type":"output"},
  "cut":{"type":"linocut","in":{"paper":"pap","flow":"fl","in":"img"},"params":{"seed":"$seed","depth":"$depth"}},
  "pap":{"params":{"colour":"#efe9dc"},"type":"paper"},
  "img":{"type":"render"},
  "fl":{"in":{"in":"img"},"type":"flow","params":{"sigma":5}}},
  "settings":{"depth":0.7,"seed":3},"name":"Linocut","format":"principia.print/1"}"##;

fn check_equal_graphs(serialise: Serialiser, a: &str, b: &str) {
    assert_eq!(
        serialise(a),
        serialise(b),
        "two equal graphs serialise differently"
    );
}

#[test]
fn qa_print_equal_graphs_serialise_equal() {
    check_equal_graphs(canonical, DOC_LINOCUT, DOC_LINOCUT_REORDERED);
    // A graph that differs in one value does not serialise equal.
    let other = DOC_LINOCUT_REORDERED.replace("\"sigma\":5", "\"sigma\":6");
    assert_ne!(
        canonical(DOC_LINOCUT),
        canonical(&other),
        "different graphs serialise equal"
    );
}

negative_control!(
    qa_print_equal_graphs_serialise_equal,
    "a serialiser that writes the authored text back",
    expected = "two equal graphs serialise differently",
    check_equal_graphs(|t| t.to_owned(), DOC_LINOCUT, DOC_LINOCUT_REORDERED)
);

fn expanded_with(text: &str, presets: &dyn Fn(&str) -> Option<Graph>) -> Result<Expanded, String> {
    let graph = Graph::parse(text).map_err(|e| format!("parse: {e}"))?;
    expand(&graph, presets).map_err(|e| e.to_string())
}

fn no_presets(_: &str) -> Option<Graph> {
    None
}

/// The doc's Linocut with the seed read as "$seed+2": depth 0.7, seed 3 + 2.
fn check_refs(text: &str) {
    let e = expanded_with(text, &no_presets).unwrap_or_else(|e| panic!("{e}"));
    let cut = &e.nodes["cut"];
    assert_eq!(
        cut.params["depth"],
        Value::Number(0.7),
        "\"$depth\" does not read the setting"
    );
    assert_eq!(
        cut.params["seed"],
        Value::Number(5.0),
        "\"$seed+2\" does not read the setting plus 2"
    );
}

#[test]
fn qa_print_dollar_references_resolve() {
    let settings: BTreeMap<String, Value> = [
        ("depth".to_owned(), Value::Number(0.7)),
        ("seed".to_owned(), Value::Number(3.0)),
    ]
    .into();
    let r = |s: &str| resolve(&Value::Text(s.into()), &settings);
    assert_eq!(r("$depth"), Ok(Value::Number(0.7)));
    assert_eq!(r("$seed+2"), Ok(Value::Number(5.0)));
    // A value that is no reference is itself.
    assert_eq!(r("#efe9dc"), Ok(Value::Text("#efe9dc".into())));
    assert_eq!(
        resolve(&Value::Number(1.5), &settings),
        Ok(Value::Number(1.5))
    );
    check_refs(&DOC_LINOCUT.replace("\"seed\": \"$seed\"", "\"seed\": \"$seed+2\""));
}

negative_control!(
    qa_print_dollar_references_resolve,
    "the graph reading \"$seed\" with no offset",
    expected = "\"$seed+2\" does not read the setting plus 2",
    check_refs(DOC_LINOCUT)
);

fn check_missing(text: &str) {
    match expanded_with(text, &no_presets) {
        Ok(_) => panic!("a graph reading \"$missing\" expanded"),
        Err(e) => assert!(
            e.contains("missing"),
            "the refusal does not name the setting: {e}"
        ),
    }
}

#[test]
fn qa_print_missing_setting_named() {
    check_missing(&DOC_LINOCUT.replace("\"$depth\"", "\"$missing\""));
    check_missing(&DOC_LINOCUT.replace("\"$depth\"", "\"$missing+1\""));
    let settings: BTreeMap<String, Value> = BTreeMap::new();
    let err = resolve(&Value::Text("$missing".into()), &settings).expect_err("resolved");
    assert!(err.contains("missing"), "{err}");
}

negative_control!(
    qa_print_missing_setting_named,
    "a graph that has the setting",
    expected = "a graph reading \"$missing\" expanded",
    check_missing(
        &DOC_LINOCUT
            .replace("\"$depth\"", "\"$missing\"")
            .replace("\"seed\": 3,", "\"seed\": 3, \"missing\": 0.5,")
    )
);

/// A graph whose middle is `{ "type": "preset:<name>" }`, its input the outer Render.
fn using(preset: &str) -> String {
    format!(
        r#"{{"format":"principia.print/1","name":"Uses {preset}","nodes":{{"src":{{"type":"render"}},
        "p":{{"type":"preset:{preset}","in":{{"in":"src"}}}},"out":{{"type":"output","in":{{"in":"p"}}}}}},"out":"out"}}"#
    )
}

/// `preset:Polaroid` runs Polaroid's graph: its one `polaroid` node appears, reading the preset node's input, its
/// `$seed` read from Polaroid's own settings, and the outer output reads it; no preset node is left.
fn check_subgraph(text: &str, presets: &dyn Fn(&str) -> Option<Graph>) {
    let e = expanded_with(text, presets).unwrap_or_else(|e| panic!("{e}"));
    assert!(
        e.nodes.values().all(|n| !n.ty.starts_with("preset:")),
        "a preset node is left after expansion"
    );
    let pol: Vec<_> = e.nodes.iter().filter(|(_, n)| n.ty == "polaroid").collect();
    assert_eq!(
        pol.len(),
        1,
        "no polaroid node: Polaroid's graph did not run as a sub-graph"
    );
    let (pid, pnode) = pol[0];
    assert_eq!(
        pnode.inputs["in"].node, "src",
        "the sub-graph does not read the preset node's input"
    );
    assert_eq!(
        pnode.params["seed"],
        Value::Number(0.0),
        "the sub-graph's $seed is not Polaroid's setting"
    );
    assert_eq!(
        &e.nodes[&e.out].inputs["in"].node, pid,
        "the outer output does not read the sub-graph"
    );
    // It validates against the registry: every wire typed.
    validate(&e, &support::registry()).unwrap_or_else(|err| panic!("{err}"));
}

#[test]
fn qa_print_preset_node_is_a_subgraph() {
    let presets = support::presets();
    let lookup = |n: &str| presets.get(n).cloned();
    check_subgraph(&using("Polaroid"), &lookup);
    // Nested: a preset that uses Polaroid, used as a node.
    let framed = Graph::parse(
        &using("Polaroid")
            .replace("Uses Polaroid", "Framed")
            .replace("\"src\"", "\"img\""),
    )
    .expect("Framed");
    let nested = |n: &str| match n {
        "Framed" => Some(framed.clone()),
        _ => presets.get(n).cloned(),
    };
    check_subgraph(&using("Framed"), &nested);
}

negative_control!(
    qa_print_preset_node_is_a_subgraph,
    "a lookup that hands back Plain for Polaroid",
    expected = "no polaroid node",
    check_subgraph(&using("Polaroid"), &|_: &str| shipped()
        .presets
        .get("Plain")
        .cloned())
);

/// Presets by name, from (name, the preset it uses).
fn chain(links: &[(&'static str, &'static str)]) -> BTreeMap<String, Graph> {
    links
        .iter()
        .map(|(name, uses)| {
            let text = using(uses)
                .replace(&format!("Uses {uses}"), name)
                .replace("\"src\"", "\"img\"");
            (
                name.to_string(),
                Graph::parse(&text).expect("a chain preset"),
            )
        })
        .collect()
}

fn check_cycle(presets: BTreeMap<String, Graph>, start: &str, names: &[&str]) {
    match expanded_with(&using(start), &|n: &str| presets.get(n).cloned()) {
        Ok(_) => panic!("a preset that includes itself expanded"),
        Err(e) => {
            for n in names {
                assert!(
                    e.contains(n),
                    "the refusal does not name `{n}` in the cycle: {e}"
                );
            }
        }
    }
}

#[test]
fn qa_print_self_including_preset_names_the_cycle() {
    check_cycle(chain(&[("Loop", "Loop")]), "Loop", &["Loop"]);
    check_cycle(
        chain(&[("Alpha", "Beta"), ("Beta", "Alpha")]),
        "Alpha",
        &["Alpha", "Beta"],
    );
    check_cycle(
        chain(&[("Alpha", "Beta"), ("Beta", "Gamma"), ("Gamma", "Alpha")]),
        "Beta",
        &["Alpha", "Beta", "Gamma"],
    );
    // The build-time check refuses it too, naming the cycle.
    let presets = chain(&[("Loop", "Loop")]);
    let text = presets["Loop"].canonical().expect("canonical");
    let err =
        check(NODES, WGSL, &[("Loop.json", text.as_str())]).expect_err("the check passed a cycle");
    assert!(err.0.contains("Loop"), "{err}");
}

negative_control!(
    qa_print_self_including_preset_names_the_cycle,
    "a chain whose last preset uses Polaroid, not the first",
    expected = "a preset that includes itself expanded",
    check_cycle(
        {
            let mut p = chain(&[("Alpha", "Beta"), ("Beta", "Polaroid")]);
            p.extend(support::presets());
            p
        },
        "Alpha",
        &["Alpha"]
    )
);

/// Each graph carries an editor position somewhere and is refused.
fn check_positions_refused(texts: &[String]) {
    for t in texts {
        if Graph::parse(t).is_ok() {
            panic!("a graph carrying editor positions parsed: {t}");
        }
    }
}

fn with_positions() -> Vec<String> {
    let node = |extra: &str| {
        DOC_LINOCUT.replace(
            r#""img":  { "type": "render" }"#,
            &format!(r#""img": {{ "type": "render", {extra} }}"#),
        )
    };
    vec![
        node(r#""pos": [120, 40]"#),
        node(r#""x": 120, "y": 40"#),
        node(r#""position": { "x": 120, "y": 40 }"#),
        node(r#""ui": { "x": 120, "y": 40 }"#),
        DOC_LINOCUT.replace(
            r#""out": "out""#,
            r#""out": "out", "positions": { "img": [0, 0] }"#,
        ),
        DOC_LINOCUT.replace(
            r#""out": "out""#,
            r#""out": "out", "editor": { "img": { "x": 0, "y": 0 } }"#,
        ),
    ]
}

#[test]
fn qa_print_editor_positions_refused() {
    let texts = with_positions();
    assert!(
        texts.iter().all(|t| t != DOC_LINOCUT),
        "a position was not inserted"
    );
    check_positions_refused(&texts);
    // The graph without them parses.
    Graph::parse(DOC_LINOCUT).expect("§4.1's Linocut parses");
}

negative_control!(
    qa_print_editor_positions_refused,
    "the graph without positions",
    expected = "a graph carrying editor positions parsed",
    check_positions_refused(&[DOC_LINOCUT.to_owned()])
);

// ───────────────────────────── REQ-RENDER-102: port types ─────────────────────────────

const TYPES: [&str; 5] = ["image", "scalar", "direction", "ground", "mask"];

const QA_PASS: &str = "qa_pass.wgsl";

fn qa_wgsl(file: &str) -> Option<String> {
    match file {
        QA_PASS => Some("fn run(uv: vec2<f32>) -> vec4<f32> {\n    return vec4<f32>(uv, 0.0, 1.0);\n}\n".into()),
        "qa_tint.wgsl" => Some(
            "// qa's tint.\nfn run(uv: vec2<f32>) -> vec4<f32> {\n    return tex(in_in, uv) * vec4<f32>(0.25, 0.5, 0.75, 1.0) * p_amount;\n}\n"
                .into(),
        ),
        _ => support::wgsl(file),
    }
}

/// A full declaration, every one of §4.3's fields present.
fn decl(
    id: &str,
    inputs: serde_json::Value,
    outputs: serde_json::Value,
    params: serde_json::Value,
    passes: &[&str],
) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "label": format!("QA {id}"),
        "category": "op",
        "doc": "A qa test entry.",
        "inputs": inputs,
        "outputs": outputs,
        "params": params,
        "passes": passes,
        "cost": "light",
        "motion": "full",
        "temporally_stable": true,
        "screen_medium": false,
        "deviations": ["A qa test entry."]
    })
}

/// The shipped registry, the fixtures', and a source and a sink of each port type.
fn typed_registry() -> Registry {
    let mut r = support::registry();
    for t in TYPES {
        let src = decl(
            &format!("qa_src_{t}"),
            serde_json::json!([]),
            serde_json::json!([{ "name": "out", "type": t }]),
            serde_json::json!([]),
            &[QA_PASS],
        );
        let sink = decl(
            &format!("qa_sink_{t}"),
            serde_json::json!([{ "name": "x", "type": t, "default": "black" }]),
            serde_json::json!([{ "name": "out", "type": "image" }]),
            serde_json::json!([]),
            &[QA_PASS],
        );
        for d in [src, sink] {
            r.register(&d.to_string(), &qa_wgsl)
                .unwrap_or_else(|e| panic!("{e}"));
        }
    }
    r
}

fn wired(from: &str, to: &str) -> Expanded {
    let text = format!(
        r#"{{"format":"principia.print/1","name":"W","nodes":{{"srcnode":{{"type":"qa_src_{from}"}},
        "sinknode":{{"type":"qa_sink_{to}","in":{{"x":"srcnode.out"}}}},"out":{{"type":"output","in":{{"in":"sinknode"}}}}}},"out":"out"}}"#
    );
    expanded_with(&text, &no_presets).unwrap_or_else(|e| panic!("{e}"))
}

type Validate = fn(&Expanded, &Registry) -> Result<Wiring, PortError>;

/// Every pair: accepted when the types match; refused otherwise, naming both ports and both types.
fn check_matrix(validate: Validate) {
    let r = typed_registry();
    for from in TYPES {
        for to in TYPES {
            match (validate(&wired(from, to), &r), from == to) {
                (Ok(_), true) => {}
                (Err(e), true) => panic!("a {from} → {to} wire was refused: {e}"),
                (Ok(_), false) => panic!("a mismatched wire was accepted: {from} → {to}"),
                (Err(e), false) => {
                    for named in ["srcnode.out", "sinknode.x", from, to] {
                        assert!(
                            e.0.contains(named),
                            "the {from} → {to} refusal does not name {named}: {e}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn qa_print_port_type_matrix() {
    check_matrix(validate);
    // The generated module refuses what validation refuses.
    let r = typed_registry();
    assert!(
        generate(&wired("image", "direction"), &r).is_err(),
        "codegen took an image → direction wire"
    );
}

negative_control!(
    qa_print_port_type_matrix,
    "a validator that accepts every wire",
    expected = "a mismatched wire was accepted",
    check_matrix(|_, _| Ok(Wiring {
        inputs: BTreeMap::new()
    }))
);

/// Node `d`'s inputs, each unconnected unless `text` wires it, read the defaults its type declares.
fn check_defaults(text: &str) {
    let mut r = typed_registry();
    let d = decl(
        "qa_defaults",
        serde_json::json!([
            { "name": "r", "type": "image", "default": "render" },
            { "name": "p", "type": "ground", "default": "paper" },
            { "name": "w", "type": "scalar", "default": "white" },
            { "name": "b", "type": "mask", "default": "black" }
        ]),
        serde_json::json!([{ "name": "out", "type": "image" }]),
        serde_json::json!([]),
        &[QA_PASS],
    );
    r.register(&d.to_string(), &qa_wgsl).expect("qa_defaults");
    let e = expanded_with(text, &no_presets).unwrap_or_else(|e| panic!("{e}"));
    let w = validate(&e, &r).unwrap_or_else(|e| panic!("{e}"));
    let feeds = &w.inputs["d"];
    for (port, want) in [
        ("r", InputDefault::Render),
        ("p", InputDefault::Paper),
        ("w", InputDefault::White),
        ("b", InputDefault::Black),
    ] {
        let got = feeds
            .iter()
            .find(|(p, _)| p == port)
            .map(|(_, f)| f.clone());
        assert_eq!(
            got,
            Some(Feed::Default(want)),
            "the unconnected input `{port}` does not take its declared default"
        );
    }
}

const DEFAULTS: &str = r#"{"format":"principia.print/1","name":"D","nodes":{"d":{"type":"qa_defaults"},
  "out":{"type":"output","in":{"in":"d"}}},"out":"out"}"#;

#[test]
fn qa_print_unconnected_inputs_take_declared_defaults() {
    check_defaults(DEFAULTS);
    // The lab's linocut defaults (§4.2): ground paper, field white.
    let text = r#"{"format":"principia.print/1","name":"L","nodes":{"img":{"type":"render"},
      "cut":{"type":"linocut","in":{"in":"img"}},"out":{"type":"output","in":{"in":"cut"}}},"out":"out"}"#;
    let e = expanded_with(text, &no_presets).expect("expands");
    let w = validate(&e, &support::registry()).expect("validates");
    let feed = |port: &str| {
        w.inputs["cut"]
            .iter()
            .find(|(p, _)| p == port)
            .map(|(_, f)| f.clone())
    };
    assert_eq!(feed("ground"), Some(Feed::Default(InputDefault::Paper)));
    assert_eq!(feed("field"), Some(Feed::Default(InputDefault::White)));
}

negative_control!(
    qa_print_unconnected_inputs_take_declared_defaults,
    "the ground input wired",
    expected = "the unconnected input `p` does not take its declared default",
    check_defaults(&DEFAULTS.replace(
        r#""d":{"type":"qa_defaults"}"#,
        r#""g":{"type":"qa_src_ground"},"d":{"type":"qa_defaults","in":{"p":"g"}}"#
    ))
);

/// Each declaration fails to register.
fn check_unregistered(decls: &[(String, serde_json::Value)]) {
    for (why, d) in decls {
        if let Ok(e) = Registry::new().register(&d.to_string(), &qa_wgsl) {
            panic!("an entry registered with {why}: `{}`", e.id);
        }
    }
}

fn port_decl(inputs: serde_json::Value, outputs: serde_json::Value) -> serde_json::Value {
    decl(
        "qa_ports",
        inputs,
        outputs,
        serde_json::json!([]),
        &[QA_PASS],
    )
}

#[test]
fn qa_print_ports_are_typed() {
    let img = serde_json::json!([{ "name": "out", "type": "image" }]);
    check_unregistered(&[
        (
            "an input of no type".into(),
            port_decl(
                serde_json::json!([{ "name": "x", "default": "black" }]),
                img.clone(),
            ),
        ),
        (
            "an input of a sixth type".into(),
            port_decl(
                serde_json::json!([{ "name": "x", "type": "colour", "default": "black" }]),
                img.clone(),
            ),
        ),
        (
            "an output of no type".into(),
            port_decl(
                serde_json::json!([]),
                serde_json::json!([{ "name": "out" }]),
            ),
        ),
        (
            "an output of a sixth type".into(),
            port_decl(
                serde_json::json!([]),
                serde_json::json!([{ "name": "out", "type": "rgba" }]),
            ),
        ),
        (
            "an input of no default".into(),
            port_decl(
                serde_json::json!([{ "name": "x", "type": "image" }]),
                img.clone(),
            ),
        ),
    ]);
    for t in TYPES {
        Registry::new()
            .register(
                &port_decl(
                    serde_json::json!([{ "name": "x", "type": t, "default": "black" }]),
                    img.clone(),
                )
                .to_string(),
                &qa_wgsl,
            )
            .unwrap_or_else(|e| panic!("an input of type {t}: {e}"));
    }
}

negative_control!(
    qa_print_ports_are_typed,
    "a well-typed entry",
    expected = "an entry registered with",
    check_unregistered(&[(
        "an image input".into(),
        port_decl(
            serde_json::json!([{ "name": "x", "type": "image", "default": "black" }]),
            serde_json::json!([{ "name": "out", "type": "image" }])
        )
    )])
);

// ───────────────────────────── REQ-RENDER-103: entries, categories, codegen ─────────────────────────────

/// §4.3's and REQ-RENDER-103's declared fields, qa's list.
const DECLARED: [&str; 13] = [
    "id",
    "label",
    "category",
    "doc",
    "inputs",
    "outputs",
    "params",
    "passes",
    "cost",
    "motion",
    "temporally_stable",
    "screen_medium",
    "deviations",
];

fn full() -> serde_json::Value {
    decl(
        "qa_full",
        serde_json::json!([{ "name": "in", "type": "image", "default": "render" }]),
        serde_json::json!([{ "name": "out", "type": "image" }]),
        serde_json::json!([{ "name": "amount", "kind": "number", "default": 0.5, "min": 0, "max": 1, "step": 0.01, "unit": "fraction" }]),
        &[QA_PASS],
    )
}

fn without(field: &str) -> serde_json::Value {
    let mut d = full();
    d.as_object_mut().expect("object").remove(field);
    d
}

fn with(field: &str, value: serde_json::Value) -> serde_json::Value {
    let mut d = full();
    d[field] = value;
    d
}

#[test]
fn qa_print_entry_lacking_any_field_fails() {
    Registry::new()
        .register(&full().to_string(), &qa_wgsl)
        .expect("the full entry registers");
    let mut cases: Vec<(String, serde_json::Value)> = DECLARED
        .iter()
        .map(|f| (format!("no `{f}`"), without(f)))
        .collect();
    cases.extend([
        ("cost `free`".into(), with("cost", "free".into())),
        ("motion `slow`".into(), with("motion", "slow".into())),
        (
            "temporally_stable \"yes\"".into(),
            with("temporally_stable", "yes".into()),
        ),
        ("screen_medium 1".into(), with("screen_medium", 1.into())),
        (
            "a missing pass file".into(),
            with("passes", serde_json::json!(["qa_nowhere.wgsl"])),
        ),
    ]);
    check_unregistered(&cases);
    // The registered entry carries each field as declared.
    let mut r = Registry::new();
    let e = r
        .register(&full().to_string(), &qa_wgsl)
        .expect("registers")
        .clone();
    assert_eq!(
        (e.id.as_str(), e.label.as_str(), e.doc.as_str()),
        ("qa_full", "QA qa_full", "A qa test entry.")
    );
    assert_eq!(e.category, Category::AdjustCombine);
    assert_eq!(
        (
            e.inputs.len(),
            e.outputs.len(),
            e.params.len(),
            e.passes.len()
        ),
        (1, 1, 1, 1)
    );
    assert_eq!(e.cost.name(), "light");
    assert_eq!(e.motion.name(), "full");
    assert!(e.temporally_stable && !e.screen_medium);
    assert_eq!(e.deviations, ["A qa test entry."]);
    for (cost, motion) in [
        ("light", "full"),
        ("medium", "reduced"),
        ("heavy", "bypass"),
    ] {
        let mut d = with("cost", cost.into());
        d["motion"] = motion.into();
        let mut r = Registry::new();
        let e = r
            .register(&d.to_string(), &qa_wgsl)
            .unwrap_or_else(|e| panic!("{cost}/{motion}: {e}"));
        assert_eq!((e.cost.name(), e.motion.name()), (cost, motion));
    }
}

negative_control!(
    qa_print_entry_lacking_any_field_fails,
    "the full entry",
    expected = "an entry registered with",
    check_unregistered(&[("every field".into(), full())])
);

/// No label reads "Source" or holds "post", and the Render node's reads "Print source".
fn check_categories(render: &str, labels: &[&str]) {
    assert_eq!(
        render, "Print source",
        "the Render node's category does not read \"Print source\""
    );
    for l in labels {
        let low = l.to_lowercase();
        assert!(
            low != "source" && !low.contains("post"),
            "a Print graph category reads {l:?}"
        );
    }
}

#[test]
fn qa_print_render_is_print_source() {
    let reg = &shipped().registry;
    let render = reg.get("render").expect("the Render node is registered");
    assert_eq!(render.label, "Render");
    assert!(
        reg.get("output").is_some(),
        "the output node is not registered"
    );
    let labels: Vec<&str> = Category::ALL.iter().map(|c| c.label()).collect();
    check_categories(render.category.label(), &labels);
    for e in reg.entries() {
        check_categories("Print source", &[e.category.label()]);
        assert!(
            !e.doc.trim().is_empty() && !e.doc.contains("\n\n"),
            "`{}`'s doc is not one paragraph",
            e.id
        );
    }
}

negative_control!(
    qa_print_render_is_print_source,
    "the lab's own label",
    expected = "the Render node's category does not read \"Print source\"",
    check_categories("Source", &["Print source"])
);

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

const TINTED: &str = r#"{"format":"principia.print/1","name":"Tinted","nodes":{"img":{"type":"render"},
  "t":{"type":"qa_tint","params":{"amount":0.3},"in":{"in":"img"}},"out":{"type":"output","in":{"in":"t"}}},"out":"out"}"#;

fn tint_registry() -> Registry {
    let mut r = shipped().registry.clone();
    let mut d = full();
    d["id"] = "qa_tint".into();
    d["passes"] = serde_json::json!(["qa_tint.wgsl"]);
    r.register(&d.to_string(), &qa_wgsl).expect("qa_tint");
    r
}

/// The module compiles, holds the tint's pass body, and has one fragment entry point per registered pass of its
/// nodes (render none, tint one, output one).
fn check_two_node(text: &str) {
    let r = tint_registry();
    let e = expanded_with(text, &no_presets).expect("expands");
    let g = generate(&e, &r).unwrap_or_else(|e| panic!("{e}"));
    let module = compile(&g.source);
    assert!(
        g.source.contains("vec4<f32>(0.25, 0.5, 0.75, 1.0)"),
        "the generated WGSL does not hold the tint's pass"
    );
    let passes: usize = e
        .nodes
        .values()
        .map(|n| r.get(&n.ty).expect("registered").passes.len())
        .sum();
    let fragments = module
        .entry_points
        .iter()
        .filter(|ep| ep.stage == naga::ShaderStage::Fragment)
        .count();
    assert_eq!(fragments, passes, "not one fragment entry point per pass");
    assert_eq!(passes, 2);
}

#[test]
fn qa_print_two_node_module_compiles() {
    check_two_node(TINTED);
}

negative_control!(
    qa_print_two_node_module_compiles,
    "a graph without the tint",
    expected = "the generated WGSL does not hold the tint's pass",
    check_two_node(&shipped().presets["Plain"].canonical().expect("canonical"))
);

fn check_stain_refuses(function: &str) {
    let node = StainNode {
        id: 0,
        output: ValueType::Vec3,
        call: Call {
            name: "qa".into(),
            function: function.into(),
            args: vec![Arg::Ctx],
            params: vec![],
        },
        inputs: vec![],
    };
    if stain_generate(Slot::Colour, &node).is_ok() {
        panic!("the stain's codegen took `{function}`");
    }
}

#[test]
fn qa_print_stain_codegen_refuses_print_passes() {
    let r = tint_registry();
    let e = expanded_with(TINTED, &no_presets).expect("expands");
    let g = generate(&e, &r).expect("generates");
    let module = compile(&g.source);
    assert!(!module.entry_points.is_empty());
    for ep in &module.entry_points {
        check_stain_refuses(&ep.name);
    }
    for p in &g.passes {
        check_stain_refuses(&p.function);
    }
}

negative_control!(
    qa_print_stain_codegen_refuses_print_passes,
    "a stain library function",
    expected = "the stain's codegen took `oklch_to_linear`",
    check_stain_refuses("oklch_to_linear")
);

// ───────────────────────────── REQ-RENDER-104: the build-time check ─────────────────────────────

fn linocut_preset(name: &str, settings: &str, depth: &str) -> String {
    format!(
        r#"{{"format":"principia.print/1","name":"{name}","settings":{settings},"nodes":{{"img":{{"type":"render"}},
        "cut":{{"type":"linocut","params":{{"depth":{depth}}},"in":{{"in":"img"}}}},"out":{{"type":"output","in":{{"in":"cut"}}}}}},"out":"out"}}"#
    )
}

/// The build-time check over the shipped files, the fixture linocut and `preset`.
fn run_check(preset: &str) -> Result<(), String> {
    let lino = support::fixture("nodes/linocut.json");
    let lino_wgsl = support::fixture("nodes/linocut.wgsl");
    let mut nodes: Vec<(&str, &str)> = NODES.to_vec();
    nodes.push(("linocut.json", lino.as_str()));
    let mut wgsl: Vec<(&str, &str)> = WGSL.to_vec();
    wgsl.push(("linocut.wgsl", lino_wgsl.as_str()));
    let mut presets: Vec<(&str, &str)> = PRESETS.to_vec();
    presets.push(("qa.json", preset));
    check(&nodes, &wgsl, &presets).map(|_| ()).map_err(|e| e.0)
}

fn check_refused(preset: &str, named: &[&str]) {
    match run_check(preset) {
        Ok(()) => panic!("the check passed a bad preset"),
        Err(e) => {
            for n in named {
                assert!(e.contains(n), "the check's error does not name {n}: {e}");
            }
        }
    }
}

#[test]
fn qa_print_check_refuses_out_of_range_through_settings() {
    check(NODES, WGSL, PRESETS).expect("the shipped registry and presets pass");
    let named = ["QA deep", "cut", "depth"];
    check_refused(
        &linocut_preset("QA deep", r#"{"depth":7}"#, "\"$depth\""),
        &named,
    );
    check_refused(
        &linocut_preset("QA deep", r#"{"depth":0.5}"#, "\"$depth+1\""),
        &named,
    );
    check_refused(
        &linocut_preset("QA deep", r#"{"depth":0.5}"#, "\"$depth-1\""),
        &named,
    );
    check_refused(&linocut_preset("QA deep", "{}", "1.01"), &named);
    check_refused(&linocut_preset("QA deep", "{}", "-0.01"), &named);
    // The range's ends, and a setting inside it, pass.
    for (s, d) in [("{}", "0"), ("{}", "1"), (r#"{"depth":0.5}"#, "\"$depth\"")] {
        run_check(&linocut_preset("QA fine", s, d))
            .unwrap_or_else(|e| panic!("depth {d} under {s}: {e}"));
    }
    // A preset naming an unknown node type, or a preset that is not there, fails naming it.
    check_refused(
        &linocut_preset("QA typo", "{}", "0.5").replace("\"linocut\"", "\"linocutt\""),
        &["QA typo", "cut", "linocutt"],
    );
    check_refused(
        &using("Nowhere").replace("Uses Nowhere", "QA lost"),
        &["QA lost", "Nowhere"],
    );
}

negative_control!(
    qa_print_check_refuses_out_of_range_through_settings,
    "a setting inside the range",
    expected = "the check passed a bad preset",
    check_refused(
        &linocut_preset("QA deep", r#"{"depth":0.7}"#, "\"$depth\""),
        &["depth"]
    )
);

fn number_param(pairs: &[(&str, serde_json::Value)]) -> serde_json::Value {
    let mut p = serde_json::json!({ "name": "radius", "kind": "number", "default": 2, "min": 0.5, "max": 8, "step": 0.5, "unit": "ref_px" });
    for (k, v) in pairs {
        if v.is_null() {
            p.as_object_mut().expect("object").remove(*k);
        } else {
            p[*k] = v.clone();
        }
    }
    let mut d = full();
    d["params"] = serde_json::json!([p]);
    d
}

#[test]
fn qa_print_parameter_needs_range_and_unit() {
    let null = serde_json::Value::Null;
    check_unregistered(&[
        (
            "a length without a unit".into(),
            number_param(&[("unit", null.clone())]),
        ),
        (
            "a unit outside §6.2's".into(),
            number_param(&[("unit", "px".into())]),
        ),
        ("no min".into(), number_param(&[("min", null.clone())])),
        ("no max".into(), number_param(&[("max", null.clone())])),
        ("no step".into(), number_param(&[("step", null.clone())])),
        (
            "no default".into(),
            number_param(&[("default", null.clone())]),
        ),
        (
            "a default outside its range".into(),
            number_param(&[("default", 9.into())]),
        ),
    ]);
    for unit in ["ref_px", "device_px", "fraction"] {
        Registry::new()
            .register(
                &number_param(&[("unit", unit.into())]).to_string(),
                &qa_wgsl,
            )
            .unwrap_or_else(|e| panic!("a length in {unit}: {e}"));
    }
    // The build-time check refuses an entry file with a unitless length, naming it.
    let bad = number_param(&[("unit", null)]).to_string();
    let mut nodes: Vec<(&str, &str)> = NODES.to_vec();
    nodes.push(("qa_full.json", bad.as_str()));
    let mut wgsl: Vec<(&str, &str)> = WGSL.to_vec();
    let pass = qa_wgsl(QA_PASS).expect("pass");
    wgsl.push((QA_PASS, pass.as_str()));
    let err = check(&nodes, &wgsl, PRESETS).expect_err("the check passed a unitless length");
    assert!(
        err.0.contains("qa_full") && err.0.contains("radius"),
        "{err}"
    );
}

negative_control!(
    qa_print_parameter_needs_range_and_unit,
    "a length in ref_px",
    expected = "an entry registered with",
    check_unregistered(&[("a length in ref_px".into(), number_param(&[]))])
);
