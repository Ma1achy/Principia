//! The Print graph's data model (TASK-M7-54; principia_print_graph.md §4.1; REQ-RENDER-101): §4.1's Linocut graph
//! parses and serialises back bitwise; `"$depth"` and `"$seed+2"` resolve to the setting and the setting plus 2;
//! `{ "type": "preset:Polaroid" }` expands to the Polaroid graph as a sub-graph; two equal graphs whose node ids differ
//! only in order serialise equal; a graph carrying editor positions is refused, a `"$missing"` reference fails naming
//! the setting, and a preset that includes itself fails naming the cycle. Each test registers its negative control
//! (R-176).

use std::collections::BTreeMap;

use render::print::graph::{expand, number, resolve, ExpandedNode, Graph, Value, Wire};
use validation::negative_control;

// This file reads the fixture presets, not the fixture registry.
#[allow(dead_code)]
#[path = "support/print_fixtures.rs"]
mod support;

/// The design doc §4.1's graph, as the doc writes it.
const LINOCUT: &str = r##"{
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

/// The same graph in the canonical serialisation: JCS, members sorted at every depth, no white space.
const LINOCUT_JCS: &str = concat!(
    r##"{"format":"principia.print/1","name":"Linocut","nodes":{"##,
    r##""cut":{"in":{"flow":"fl","in":"img","paper":"pap"},"params":{"depth":"$depth","seed":"$seed"},"type":"linocut"},"##,
    r##""fl":{"in":{"in":"img"},"params":{"sigma":5},"type":"flow"},"##,
    r##""img":{"type":"render"},"##,
    r##""out":{"in":{"in":"cut"},"type":"output"},"##,
    r##""pap":{"params":{"colour":"#efe9dc"},"type":"paper"}},"##,
    r##""out":"out","settings":{"depth":0.7,"seed":3}}"##
);

fn parse(text: &str) -> Graph {
    Graph::parse(text).unwrap_or_else(|e| panic!("the graph does not parse: {e}"))
}

fn canonical(g: &Graph) -> String {
    g.canonical().expect("a parsed graph has a canonical form")
}

// ----- round trip -----

/// `text` serialises, through `serialise`, as `want`, and `want` parses and serialises back to itself, bitwise.
fn check_round_trip(text: &str, want: &str, serialise: fn(&Graph) -> String) {
    let first = serialise(&parse(text));
    assert_eq!(first, want, "the graph does not serialise canonically");
    let again = serialise(&parse(&first));
    assert_eq!(again, first, "the graph does not serialise back bitwise");
}

#[test]
fn print_graph_model_linocut_round_trips_bitwise() {
    check_round_trip(LINOCUT, LINOCUT_JCS, canonical);
    // A graph without settings is written with `"settings":{}`, and that text is canonical in its turn.
    let bare =
        r#"{"format":"principia.print/1","name":"P","nodes":{"o":{"type":"output"}},"out":"o"}"#;
    let want = r#"{"format":"principia.print/1","name":"P","nodes":{"o":{"type":"output"}},"out":"o","settings":{}}"#;
    check_round_trip(bare, want, canonical);
    // Strings are escaped as JCS escapes them (RFC 8785 §3.2.2.2): a space as itself, U+001F and U+0001 as `\u00xx`.
    let name = "A b\u{1f}\u{1}\u{8}\t\n\u{c}\r\"\\é";
    let g = Graph {
        name: name.into(),
        ..parse(bare)
    };
    let text = canonical(&g);
    assert!(
        text.contains(r#""name":"A b\u001f\u0001\b\t\n\f\r\"\\é""#),
        "the name is not escaped as JCS escapes it: {text}"
    );
    assert_eq!(canonical(&parse(&text)), text);
}

negative_control!(
    print_graph_model_linocut_round_trips_bitwise,
    "serde_json's own writer, which keeps no JCS number form and so writes 5.0 for 5",
    expected = "the graph does not serialise canonically",
    check_round_trip(LINOCUT, LINOCUT_JCS, |g| {
        let mut nodes = serde_json::Map::new();
        for (id, n) in &g.nodes {
            let params: serde_json::Map<String, serde_json::Value> = n
                .params
                .iter()
                .map(|(k, v)| {
                    let v = match v {
                        Value::Number(x) => serde_json::json!(x),
                        Value::Text(t) => serde_json::json!(t),
                    };
                    (k.clone(), v)
                })
                .collect();
            nodes.insert(
                id.clone(),
                serde_json::json!({ "type": n.ty, "params": params, "in": n.inputs }),
            );
        }
        serde_json::json!({ "format": "principia.print/1", "name": g.name, "nodes": nodes, "out": g.out })
            .to_string()
    })
);

/// Each number is written in JCS's form (RFC 8785 §3.2.2.3), through `format`.
fn check_numbers(format: fn(f64) -> String) {
    let cases = [
        (0.0, "0"),
        (-0.0, "0"),
        (5.0, "5"),
        (-3.0, "-3"),
        (0.7, "0.7"),
        (123.456, "123.456"),
        (1e20, "100000000000000000000"),
        (1e21, "1e+21"),
        (1.5e21, "1.5e+21"),
        (1e-6, "0.000001"),
        (1.5e-6, "0.0000015"),
        (1e-7, "1e-7"),
        (1.25e-7, "1.25e-7"),
        (123_456_789_012_345_680_000.0, "123456789012345680000"),
        (0.1 + 0.2, "0.30000000000000004"),
    ];
    for (v, want) in cases {
        assert_eq!(format(v), want, "{v:e} is not written in JCS's form");
    }
}

#[test]
fn print_graph_model_numbers_are_jcs() {
    check_numbers(|v| number(v).expect("a finite number"));
    for v in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(number(v).is_err(), "{v} has no JSON form, yet was written");
    }
    let nan = Graph {
        name: "N".into(),
        settings: BTreeMap::from([("a".into(), Value::Number(f64::NAN))]),
        nodes: BTreeMap::new(),
        out: "o".into(),
    };
    assert!(nan.canonical().is_err(), "a NaN setting was serialised");
}

negative_control!(
    print_graph_model_numbers_are_jcs,
    "Rust's own float formatting, which writes 1e21 in full",
    expected = "is not written in JCS's form",
    check_numbers(|v| v.to_string())
);

// ----- `$` references -----

fn settings() -> BTreeMap<String, Value> {
    BTreeMap::from([
        ("depth".to_owned(), Value::Number(0.7)),
        ("seed".to_owned(), Value::Number(3.0)),
        ("ink".to_owned(), Value::Text("#171414".to_owned())),
    ])
}

type Resolver = fn(&Value, &BTreeMap<String, Value>) -> Result<Value, String>;

/// `"$depth"` reads the setting, `"$seed+2"` the setting plus 2 and `"$seed-1"` less 1; a literal is itself; an offset
/// that is no decimal integer, or on a setting that is no number, is refused.
fn check_resolve(resolve: Resolver) {
    let s = settings();
    let text = |t: &str| Value::Text(t.to_owned());
    let read = |t: &str| resolve(&text(t), &s);
    assert_eq!(
        read("$depth"),
        Ok(Value::Number(0.7)),
        "\"$depth\" does not read the setting"
    );
    assert_eq!(
        read("$seed+2"),
        Ok(Value::Number(5.0)),
        "\"$seed+2\" is not the setting plus 2"
    );
    assert_eq!(
        read("$seed-1"),
        Ok(Value::Number(2.0)),
        "\"$seed-1\" is not the setting less 1"
    );
    assert_eq!(
        read("$seed+0"),
        Ok(Value::Number(3.0)),
        "\"$seed+0\" is not the setting"
    );
    assert_eq!(
        read("$ink"),
        Ok(text("#171414")),
        "\"$ink\" does not read the setting"
    );
    assert_eq!(
        read("#efe9dc"),
        Ok(text("#efe9dc")),
        "a literal is not itself"
    );
    assert_eq!(resolve(&Value::Number(4.0), &s), Ok(Value::Number(4.0)));
    for bad in [
        "$seed+",
        "$seed+x",
        "$seed++2",
        "$seed+-2",
        "$seed+2.5",
        "$ink+1",
    ] {
        assert!(read(bad).is_err(), "{bad:?} was resolved");
    }
    assert_eq!(
        read("$ink+1"),
        Err(
            "\"$ink+1\": the setting `ink` = \"#171414\" is no number, so it takes no offset"
                .to_owned()
        )
    );
}

#[test]
fn print_graph_model_settings_resolve() {
    check_resolve(resolve);
}

negative_control!(
    print_graph_model_settings_resolve,
    "a resolver that leaves every value as written",
    expected = "\"$depth\" does not read the setting",
    check_resolve(|v, _| Ok(v.clone()))
);

/// `"$missing"` fails, naming the setting, both resolved alone and when the graph expands.
fn check_missing(graph: &str) {
    let err = resolve(&Value::Text("$missing".into()), &settings())
        .expect_err("\"$missing\" was resolved");
    assert!(err.contains("`missing`"), "{err}");
    let err = expand(&parse(graph), &|_: &str| None)
        .expect_err("a graph reading a setting it does not have expanded");
    assert_eq!(err.to_string(), err.0);
    assert!(
        err.0.contains("setting `missing`") && err.0.contains("`cut`") && err.0.contains("`depth`"),
        "the error does not name the setting, the node and the parameter: {err}"
    );
}

const MISSING: &str = r#"{"format":"principia.print/1","name":"M","settings":{"seed":1},
  "nodes":{"img":{"type":"render"},"cut":{"type":"linocut","params":{"depth":"$missing"},"in":{"in":"img"}},
           "out":{"type":"output","in":{"in":"cut"}}},"out":"out"}"#;

#[test]
fn print_graph_model_missing_setting_named() {
    check_missing(MISSING);
}

negative_control!(
    print_graph_model_missing_setting_named,
    "a graph that has the setting",
    expected = "a graph reading a setting it does not have expanded",
    check_missing(&MISSING.replace("\"seed\":1", "\"missing\":1"))
);

// ----- presets as sub-graphs -----

/// The graph using the fixture Polaroid as its one node between the render and the output.
const ON_POLAROID: &str = r#"{"format":"principia.print/1","name":"On a Polaroid","settings":{"s":2},
  "nodes":{"img":{"type":"render"},"pol":{"type":"preset:Polaroid","params":{"seed":"$s+2"},"in":{"in":"img"}},
           "out":{"type":"output","in":{"in":"pol"}}},"out":"out"}"#;

fn wire(node: &str) -> Wire {
    Wire {
        node: node.to_owned(),
        port: None,
    }
}

/// `{ "type": "preset:Polaroid" }` expands, through `presets`, to the Polaroid graph as a sub-graph: its node `pol`
/// as `pol/pol`, its setting the preset node's parameter, its Render node the preset node's input, its output node
/// dropped and the outer output reading what fed it.
fn check_expansion(presets: &dyn Fn(&str) -> Option<Graph>) {
    let got = expand(&parse(ON_POLAROID), presets)
        .map(|e| e.nodes)
        .unwrap_or_default();
    let node = |ty: &str, params: &[(&str, f64)], inputs: &[(&str, &str)], entry: Option<&str>| {
        ExpandedNode {
            ty: ty.to_owned(),
            params: params
                .iter()
                .map(|(k, v)| ((*k).to_owned(), Value::Number(*v)))
                .collect(),
            inputs: inputs
                .iter()
                .map(|(p, n)| ((*p).to_owned(), wire(n)))
                .collect(),
            entry: entry.map(wire),
        }
    };
    let want = BTreeMap::from([
        ("img".to_owned(), node("render", &[], &[], None)),
        (
            "out".to_owned(),
            node("output", &[], &[("in", "pol/pol")], None),
        ),
        (
            "pol/pol".to_owned(),
            node("polaroid", &[("seed", 4.0)], &[("in", "img")], Some("img")),
        ),
    ]);
    assert_eq!(
        got, want,
        "the preset node does not expand to the Polaroid graph"
    );
}

#[test]
fn print_graph_model_preset_expands_as_sub_graph() {
    let presets = support::presets();
    check_expansion(&|n: &str| presets.get(n).cloned());
    // Unwired, the preset node's sub-graph keeps its own Render node, and its parameters keep the preset's settings.
    let unwired = ON_POLAROID
        .replace(r#","params":{"seed":"$s+2"}"#, "")
        .replace(r#","in":{"in":"img"}"#, "");
    let e = expand(&parse(&unwired), &|n: &str| presets.get(n).cloned()).expect("expands");
    assert_eq!(e.nodes["pol/img"].ty, "render");
    assert_eq!(e.nodes["pol/pol"].inputs["in"], wire("pol/img"));
    assert_eq!(e.nodes["pol/pol"].params["seed"], Value::Number(0.0));
    assert_eq!(e.nodes["out"].inputs["in"], wire("pol/pol"));
    // A preset is found by name, or refused naming it; a parameter its preset has no setting for is refused.
    let err = expand(&parse(ON_POLAROID), &|_: &str| None).expect_err("no preset, yet expanded");
    assert!(err.0.contains("no preset `Polaroid`"), "{err}");
    let typo = ON_POLAROID.replace(r#""seed":"$s+2""#, r#""sed":1"#);
    let err = expand(&parse(&typo), &|n: &str| presets.get(n).cloned()).expect_err("expanded");
    assert!(err.0.contains("no setting `sed`"), "{err}");
    check_preset_wiring(&presets);
}

/// A preset node's output read as `pol` or `pol.out`, never another port; its one input `in`; chained preset nodes;
/// a preset node reading its own output; and a preset whose own preset node reads its Render node, the outer input.
fn check_preset_wiring(presets: &BTreeMap<String, Graph>) {
    let lookup = |n: &str| presets.get(n).cloned();
    let out_of =
        |text: &str| expand(&parse(text), &lookup).map(|e| e.nodes["out"].inputs["in"].clone());
    assert_eq!(
        out_of(&ON_POLAROID.replace(r#"{"in":"pol"}"#, r#"{"in":"pol.out"}"#)),
        Ok(wire("pol/pol"))
    );
    let err = out_of(&ON_POLAROID.replace(r#"{"in":"pol"}"#, r#"{"in":"pol.grain"}"#))
        .expect_err("pol.grain");
    assert!(err.0.contains("has one output, `out`"), "{err}");
    let err = out_of(&ON_POLAROID.replace(r#""in":{"in":"img"}"#, r#""in":{"src":"img"}"#))
        .expect_err("src");
    assert!(err.0.contains("a preset node has one input, `in`"), "{err}");
    let chained = r#"{"format":"principia.print/1","name":"Two","nodes":{"img":{"type":"render"},
      "a":{"type":"preset:Polaroid","in":{"in":"img"}},"b":{"type":"preset:Polaroid","in":{"in":"a"}},
      "out":{"type":"output","in":{"in":"b"}}},"out":"out"}"#;
    let e = expand(&parse(chained), &lookup).expect("two presets in a chain expand");
    assert_eq!(e.nodes["b/pol"].inputs["in"], wire("a/pol"));
    assert_eq!(e.nodes["out"].inputs["in"], wire("b/pol"));
    let own = chained.replace(
        r#""a":{"type":"preset:Polaroid","in":{"in":"img"}}"#,
        r#""a":{"type":"preset:Polaroid","in":{"in":"a"}}"#,
    );
    let err = expand(&parse(&own), &lookup).expect_err("a preset node reading itself expanded");
    assert!(
        err.0.contains("node `a`: its preset reads its own output"),
        "{err}"
    );
    // Wrap runs Polaroid on its own input: inside it, the Render node is Wrap's input, and the preset node is not.
    let wrap = parse(
        r#"{"format":"principia.print/1","name":"Wrap","nodes":{"r":{"type":"render"},
            "q":{"type":"preset:Polaroid","in":{"in":"r"}},"out":{"type":"output","in":{"in":"q"}}},"out":"out"}"#,
    );
    let mut more = presets.clone();
    more.insert("Wrap".into(), wrap);
    let outer = ON_POLAROID
        .replace("preset:Polaroid", "preset:Wrap")
        .replace(r#","params":{"seed":"$s+2"}"#, "");
    let e = expand(&parse(&outer), &|n: &str| more.get(n).cloned()).expect("Wrap expands");
    assert_eq!(
        e.nodes["out"].inputs["in"],
        wire("pol/q/pol"),
        "the preset node read as its input"
    );
    assert_eq!(e.nodes["pol/q/pol"].inputs["in"], wire("img"));
    assert_eq!(e.nodes["pol/q/pol"].entry, Some(wire("img")));
    // A sub-graph's node may not read the sub-graph's output node, which expansion drops.
    let reads_out = parse(
        r#"{"format":"principia.print/1","name":"Polaroid","settings":{"seed":0},"nodes":{"m":{"type":"polaroid",
            "in":{"in":"out"}},"out":{"type":"output","in":{"in":"m"}}},"out":"out"}"#,
    );
    let err = expand(&parse(ON_POLAROID), &|_: &str| Some(reads_out.clone()))
        .expect_err("read its output");
    assert!(err.0.contains("reads the preset's output node"), "{err}");
}

negative_control!(
    print_graph_model_preset_expands_as_sub_graph,
    "a Polaroid preset whose medium is wired from nothing: the sub-graph's Render node not replaced by the input",
    expected = "the preset node does not expand to the Polaroid graph",
    check_expansion(&|_: &str| Some(parse(
        r#"{"format":"principia.print/1","name":"Polaroid","settings":{"seed":0},
            "nodes":{"pol":{"type":"polaroid","params":{"seed":"$seed"}},"out":{"type":"output","in":{"in":"pol"}}},
            "out":"out"}"#
    )))
);

/// A preset that includes itself, directly or through another, fails naming the cycle.
fn check_cycle(presets: &BTreeMap<String, Graph>, start: &str, cycle: &str) {
    let err = expand(&presets[start], &|n: &str| presets.get(n).cloned())
        .expect_err("a preset that includes itself expanded");
    assert!(
        err.0.contains(cycle),
        "the error does not name the cycle {cycle}: {err}"
    );
}

fn including(name: &str, includes: &str) -> Graph {
    parse(&format!(
        r#"{{"format":"principia.print/1","name":"{name}","nodes":{{"p":{{"type":"preset:{includes}"}},
            "out":{{"type":"output","in":{{"in":"p"}}}}}},"out":"out"}}"#
    ))
}

fn cyclic() -> BTreeMap<String, Graph> {
    BTreeMap::from([
        ("Loop".to_owned(), including("Loop", "Loop")),
        ("A".to_owned(), including("A", "B")),
        ("B".to_owned(), including("B", "A")),
        ("C".to_owned(), including("C", "Plain")),
        ("Plain".to_owned(), support::presets()["Plain"].clone()),
    ])
}

#[test]
fn print_graph_model_self_including_preset_names_cycle() {
    let presets = cyclic();
    check_cycle(&presets, "Loop", "Loop → Loop");
    // Expanding A, the cycle is met inside it: B includes A, which includes B.
    check_cycle(&presets, "A", "B → A → B");
    // A preset including another, with no cycle, expands.
    let c = expand(&presets["C"], &|n: &str| presets.get(n).cloned()).expect("C expands");
    assert_eq!(c.nodes["out"].inputs["in"], wire("p/render"));
}

negative_control!(
    print_graph_model_self_including_preset_names_cycle,
    "a preset that includes another, with no cycle",
    expected = "a preset that includes itself expanded",
    check_cycle(&cyclic(), "C", "C → C")
);

// ----- node ids carry no meaning -----

const ORDER_A: &str = r#"{"format":"principia.print/1","name":"G","settings":{"b":1,"a":2},
  "nodes":{"img":{"type":"render"},"cut":{"type":"linocut","in":{"paper":"img","in":"img"}},
           "out":{"type":"output","in":{"in":"cut"}}},"out":"out"}"#;
const ORDER_B: &str = r#"{"out":"out","nodes":{"out":{"in":{"in":"cut"},"type":"output"},
  "cut":{"in":{"in":"img","paper":"img"},"type":"linocut"},"img":{"type":"render"}},
  "settings":{"a":2,"b":1},"name":"G","format":"principia.print/1"}"#;

/// Two graphs equal but for the order their node ids and members are written in serialise equal.
fn check_order(a: &str, b: &str) {
    assert_eq!(
        canonical(&parse(a)),
        canonical(&parse(b)),
        "two equal graphs do not serialise equal"
    );
}

#[test]
fn print_graph_model_node_order_carries_no_meaning() {
    check_order(ORDER_A, ORDER_B);
    // Sorted by UTF-16 code units, as JCS sorts (RFC 8785 §3.2.3): U+1F600, its surrogates D83D DE00, before U+FB01,
    // which sorts first by UTF-8 (EF AC 81 against F0 9F 98 80).
    let g = parse(
        r#"{"format":"principia.print/1","name":"U","nodes":{"😀":{"type":"render"},"ﬁ":{"type":"output","in":{"in":"😀"}}},"out":"ﬁ"}"#,
    );
    let text = canonical(&g);
    let at = |id: &str| {
        text.find(&format!("\"{id}\":{{"))
            .expect("the node is written")
    };
    assert!(
        at("😀") < at("ﬁ"),
        "the node ids are not in UTF-16 order: {text}"
    );
}

negative_control!(
    print_graph_model_node_order_carries_no_meaning,
    "two graphs that differ in a node's id, not only in order",
    expected = "two equal graphs do not serialise equal",
    check_order(
        ORDER_A,
        &ORDER_B.replace("\"paper\":\"img\"", "\"flow\":\"img\"")
    )
);

// ----- refusals -----

/// Each of `cases` is refused, its error holding the text given with it.
fn check_refused(cases: &[(&str, &str)]) {
    for (text, why) in cases {
        match Graph::parse(text) {
            Ok(_) => panic!("a graph that must be refused parsed: {text}"),
            Err(e) => assert!(
                e.0.contains(why),
                "{text}: the error {e} does not say {why:?}"
            ),
        }
    }
}

const POSITIONED: &str = r#"{"format":"principia.print/1","name":"G","nodes":{"img":{"type":"render","x":30,"y":40},
  "out":{"type":"output","in":{"in":"img"}}},"out":"out"}"#;

#[test]
fn print_graph_model_refuses_editor_positions() {
    check_refused(&[
        (POSITIONED, "editor position, `x`"),
        (
            &POSITIONED.replace("\"x\":30,\"y\":40", "\"position\":[1,2]"),
            "editor position",
        ),
        (
            &POSITIONED.replace("\"x\":30,\"y\":40", "\"pos\":[1,2]"),
            "editor position",
        ),
        (&POSITIONED.replace(",\"x\":30", ""), "editor position, `y`"),
        (
            &POSITIONED.replace("\"out\":\"out\"", "\"out\":\"out\",\"positions\":{}"),
            "`positions`",
        ),
    ]);
    // Every other member a node or a graph does not have is refused too, and each member missing or ill-formed.
    let ok = POSITIONED.replace(",\"x\":30,\"y\":40", "");
    parse(&ok);
    check_refused(&[
        ("[]", "is a JSON object"),
        ("{", "not JSON"),
        (
            &ok.replace("\"type\":\"render\"", "\"type\":\"render\",\"doc\":1"),
            "`doc`, which is no member of a node",
        ),
        (
            &ok.replace("principia.print/1", "principia.print/2"),
            "the format",
        ),
        (&ok.replace("\"name\":\"G\",", ""), "no string `name`"),
        (&ok.replace(",\"out\":\"out\"}", "}"), "no string `out`"),
        (
            &ok.replace("\"out\":\"out\"}", "\"out\":\"nowhere\"}"),
            "`out` names `nowhere`",
        ),
        (
            &ok.replace("{\"in\":\"img\"}", "{\"in\":\"gone\"}"),
            "reads `gone`, which is no node",
        ),
        (
            &ok.replace("{\"in\":\"img\"}", "{\"in\":3}"),
            "not `node` or `node.port`",
        ),
        (
            &ok.replace("{\"in\":\"img\"}", "[]"),
            "`in` is not an object",
        ),
        (&ok.replace("\"img\":{", "\"i.m\":{"), "holds `.` or `/`"),
        (&ok.replace("\"img\":{", "\"i/m\":{"), "holds `.` or `/`"),
        (&ok.replace("\"img\":{", "\"\":{"), "is empty"),
        (
            &ok.replace(
                "\"type\":\"render\"",
                "\"type\":\"render\",\"params\":{\"a\":true}",
            ),
            "neither a number nor a string",
        ),
        (
            &ok.replace("\"type\":\"render\"", "\"type\":\"render\",\"params\":[]"),
            "params is not an object",
        ),
        (
            &ok.replace("\"name\":\"G\",", "\"name\":\"G\",\"settings\":[],"),
            "settings is not an object",
        ),
        (
            &ok.replace("\"img\":{\"type\":\"render\"}", "\"img\":3"),
            "node `img` is not an object",
        ),
        (
            &ok.replace("{\"type\":\"render\"}", "{}"),
            "no string `type`",
        ),
        (&ok.replace("\"nodes\":", "\"nodez\":"), "`nodez`"),
    ]);
    // A wire may name a node's port.
    parse(&ok.replace("{\"in\":\"img\"}", "{\"in\":\"img.out\"}"));
}

negative_control!(
    print_graph_model_refuses_editor_positions,
    "the same graph without its positions",
    expected = "a graph that must be refused parsed",
    check_refused(&[(
        &POSITIONED.replace(",\"x\":30,\"y\":40", ""),
        "editor position"
    )])
);
