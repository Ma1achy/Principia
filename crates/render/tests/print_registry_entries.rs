//! The Print graph's registry entries and codegen (TASK-M7-54; principia_print_graph.md §4.3; render_gui_spec §12;
//! REQ-RENDER-103): every registered entry carries each declared field; the Render node's category reads "Print
//! source" and no Print graph category reads "Source" or "post"; the generated WGSL of a two-node graph compiles under
//! naga and holds each node's passes; an entry without a motion policy, and one without `temporally_stable`, each fail
//! to register; a pass's nested block comments are skipped whole, and one never closed is refused; and the stain's
//! codegen given a Print graph node refuses it. Each test registers its negative control (R-176).

use naga::valid::{Capabilities, ValidationFlags, Validator};
use render::codegen::{generate as stain_generate, Arg, Call, Node as StainNode, Slot, ValueType};
use render::print::codegen::{
    generate, Generated, Reads, FRAME_GROUP, PREFIX, TEXTURE_GROUP, UNIFORM_GROUP,
};
use render::print::graph::{expand, Graph};
use render::print::ports::InputDefault;
use render::print::registry::{Category, CostClass, MotionPolicy, Registry, Unit, FIELDS};
use render::print::{shipped, NODES};
use validation::negative_control;

#[path = "support/print_fixtures.rs"]
mod support;

// ----- every field -----

/// Each declaration of `decls` carries every member [`FIELDS`] lists, and registers.
fn check_fields(decls: &[(&str, &str)]) {
    for (file, decl) in decls {
        let json: serde_json::Value = serde_json::from_str(decl).expect("a declaration is JSON");
        for (key, what) in FIELDS {
            assert!(json.get(key).is_some(), "{file} lacks `{key}` ({what})");
        }
        Registry::new()
            .register(decl, &support::wgsl)
            .unwrap_or_else(|e| panic!("{file}: {e}"));
    }
}

#[test]
fn print_registry_entries_carry_every_field() {
    check_fields(NODES);
    let registry = &shipped().registry;
    let ids: Vec<&str> = registry.entries().map(|e| e.id.as_str()).collect();
    assert_eq!(
        ids,
        ["output", "render"],
        "the shipped registry is not Render and Output"
    );
    for e in registry.entries() {
        assert!(
            !e.label.is_empty() && !e.doc.is_empty(),
            "{} has no label or doc",
            e.id
        );
        assert_eq!(e.cost, CostClass::Light, "{}", e.id);
        assert_eq!(e.motion, MotionPolicy::Full, "{}", e.id);
        assert!(e.temporally_stable && !e.screen_medium, "{}", e.id);
    }
    let render = registry.get("render").expect("render");
    assert!(render.inputs.is_empty() && render.passes.is_empty() && render.params.is_empty());
    assert_eq!(render.outputs.len(), 1);
    assert_eq!(
        render.deviations.len(),
        1,
        "render lists its id and category"
    );
    let output = registry.get("output").expect("output");
    assert!(output.outputs.is_empty() && output.deviations.is_empty());
    assert_eq!(
        (output.inputs[0].name.as_str(), output.inputs[0].default),
        ("in", InputDefault::Render)
    );
    assert_eq!(output.passes[0].file, "output.wgsl");
    // Every field read from a fixture entry: its kinds, ranges, steps and units.
    let all = support::registry();
    let lino = all.get("linocut").expect("the fixture linocut");
    assert_eq!(lino.category, Category::Print);
    assert_eq!(
        (lino.cost, lino.motion),
        (CostClass::Medium, MotionPolicy::Reduced)
    );
    assert_eq!(
        lino.param("gouge").map(|p| p.kind.clone()),
        Some(render::print::registry::ParamKind::Number {
            min: 0.5,
            max: 4.0,
            step: 0.05,
            unit: Unit::RefPx,
        })
    );
    for u in Unit::ALL {
        assert!(!u.name().is_empty());
    }
}

negative_control!(
    print_registry_entries_carry_every_field,
    "a declaration without its motion policy",
    expected = "lacks `motion`",
    check_fields(&[(
        "output.json",
        &NODES[0].1.replace("\"motion\": \"full\",", "")
    )])
);

// ----- categories -----

/// Render's category reads "Print source", and none of `labels` reads "Source" or holds "post".
fn check_categories(render: &str, labels: &[&str]) {
    assert_eq!(
        render, "Print source",
        "the Render node's category does not read \"Print source\""
    );
    for label in labels {
        assert!(
            *label != "Source" && !label.to_lowercase().contains("post"),
            "a Print graph category reads {label:?}"
        );
    }
}

#[test]
fn print_registry_entries_render_is_print_source() {
    let registry = &shipped().registry;
    let render = registry.get("render").expect("render").category;
    assert_eq!(render, Category::PrintSource);
    let mut labels: Vec<&str> = Category::ALL.iter().map(|c| c.label()).collect();
    labels.extend(registry.entries().map(|e| e.category.label()));
    check_categories(render.label(), &labels);
    for c in Category::ALL {
        assert_eq!(Category::parse(c.key()), Some(c));
        assert!(
            !c.key().contains("post") && c.key() != "source",
            "{}",
            c.key()
        );
    }
    assert_eq!(
        Category::parse("source"),
        None,
        "the lab's key `source` is read"
    );
}

negative_control!(
    print_registry_entries_render_is_print_source,
    "the lab's own label, \"Source\", among the categories",
    expected = "a Print graph category reads \"Source\"",
    check_categories("Print source", &["Generate", "Source"])
);

// ----- codegen -----

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

fn generated(text: &str) -> Generated {
    let graph = Graph::parse(text).expect("the graph parses");
    let presets = support::presets();
    let e = expand(&graph, &|n: &str| presets.get(n).cloned()).expect("the graph expands");
    generate(&e, &support::registry())
        .unwrap_or_else(|e| panic!("the graph does not generate: {e}"))
}

/// `source` compiles under naga and holds each of `passes`, each a function and its fragment entry point.
fn check_module(source: &str, passes: &[&str]) {
    compile(source);
    for p in passes {
        assert!(
            source.contains(&format!("fn {p}(uv: vec2<f32>) -> vec4<f32>"))
                && source.contains(&format!("@fragment\nfn {p}_main(")),
            "the generated WGSL does not hold the pass {p}"
        );
    }
}

const BLURRED: &str = r#"{"format":"principia.print/1","name":"Blurred","nodes":{"img":{"type":"render"},
  "b":{"type":"blur","params":{"sigma":3},"in":{"in":"img"}},"out":{"type":"output","in":{"in":"b"}}},"out":"out"}"#;

#[test]
fn print_registry_entries_two_node_graph_compiles() {
    // Plain, the two shipped nodes: Render has no pass (the stain's colours are its texture), Output one.
    let plain = generated(&shipped().presets["Plain"].canonical().expect("canonical"));
    assert_eq!(plain.order, ["render", "out"]);
    check_module(&plain.source, &["print_n1_pass0"]);
    assert!(
        plain
            .source
            .contains("return print_tex(print_n1_in_in, uv);"),
        "{}",
        plain.source
    );
    assert_eq!(plain.passes.len(), 1);
    assert_eq!(
        (
            plain.passes[0].node.as_str(),
            plain.passes[0].ty.as_str(),
            plain.passes[0].entry_point.as_str()
        ),
        ("out", "output", "print_n1_pass0_main")
    );
    assert_eq!(plain.textures.len(), 1);
    assert_eq!(
        (plain.textures[0].binding, plain.textures[0].reads.clone()),
        (0, Reads::Source)
    );
    assert!(plain.uniforms.is_empty());
    assert!(plain.source.contains(&format!(
        "@group({FRAME_GROUP}) @binding(0) var<uniform> print_frame"
    )));
    // A two-pass node between them: its passes in order, the second reading the first through `prev`.
    let blurred = generated(BLURRED);
    check_module(
        &blurred.source,
        &["print_n1_pass0", "print_n1_pass1", "print_n2_pass0"],
    );
    assert!(
        blurred.source.contains("print_tex(print_n1_prev, uv - d)"),
        "{}",
        blurred.source
    );
    assert!(
        blurred
            .source
            .contains("print_frame.px.x * print_n1.p_sigma"),
        "{}",
        blurred.source
    );
    // Comments keep the lab's names as written.
    for comment in [
        "// A test fixture's second pass: down, reading the first through prev, not in_in.\nfn print_n1_pass1(",
        "print_n1.p_sigma); /* the lab's names, kept in a comment: p_sigma, px*/\n",
    ] {
        assert!(blurred.source.contains(comment), "the comment is not kept as written: {}", blurred.source);
    }
    let reads: Vec<(u32, &str, Reads)> = blurred
        .textures
        .iter()
        .map(|t| (t.binding, t.name.as_str(), t.reads.clone()))
        .collect();
    assert_eq!(
        reads,
        [
            (0, "print_n1_in_in", Reads::Source),
            (1, "print_n1_prev", Reads::Previous { node: "b".into() }),
            (
                2,
                "print_n2_in_in",
                Reads::Node {
                    node: "b".into(),
                    port: "out".into()
                }
            ),
        ]
    );
    assert_eq!(blurred.uniforms.len(), 1);
    assert_eq!(blurred.uniforms[0].binding, 1);
    assert_eq!(
        blurred.uniforms[0].values,
        [("sigma".to_owned(), vec![3.0])]
    );
    assert!(blurred.source.contains(&format!(
        "@group({UNIFORM_GROUP}) @binding(1) var<uniform> print_n1: PrintN1;"
    )));
    assert!(blurred.source.contains(&format!(
        "@group({TEXTURE_GROUP}) @binding(1) var print_n1_prev"
    )));
}

negative_control!(
    print_registry_entries_two_node_graph_compiles,
    "the module with its output pass taken out",
    expected = "the generated WGSL does not hold the pass print_n1_pass0",
    {
        let plain = generated(&shipped().presets["Plain"].canonical().expect("canonical"));
        let cut = plain
            .source
            .replace("fn print_n1_pass0(", "fn print_n1_gone(")
            .replace(
                "return print_n1_pass0(frag_uv",
                "return print_n1_gone(frag_uv",
            );
        check_module(&cut, &["print_n1_pass0"])
    }
);

/// The fixture linocut's uniforms, textures and renamed pass.
#[test]
fn print_registry_entries_codegen_binds_inputs_and_params() {
    let g = generated(
        r##"{"format":"principia.print/1","name":"L","settings":{"s":3},"nodes":{"img":{"type":"render"},
          "cut":{"type":"linocut","params":{"seed":"$s+2","ink":"#ff8000","cut":1},"in":{"in":"img"}},
          "out":{"type":"output","in":{"in":"cut"}}},"out":"out"}"##,
    );
    check_linocut(&g);
}

fn check_linocut(g: &Generated) {
    compile(&g.source);
    let reads: Vec<Reads> = g.textures.iter().map(|t| t.reads.clone()).collect();
    assert_eq!(
        reads,
        [
            Reads::Source,
            Reads::Default(InputDefault::Black),
            Reads::Default(InputDefault::Paper),
            Reads::Default(InputDefault::Paper),
            Reads::Default(InputDefault::White),
            Reads::Node {
                node: "cut".into(),
                port: "out".into()
            },
        ],
        "the linocut's textures do not read their wires and defaults"
    );
    let values = &g.uniforms[0].values;
    let want: Vec<(String, Vec<f32>)> = vec![
        ("depth".into(), vec![0.7]),
        ("gouge".into(), vec![1.1]),
        ("seed".into(), vec![5.0]),
        ("ink".into(), vec![1.0, 128.0 / 255.0, 0.0]),
        ("cut".into(), vec![1.0]),
    ];
    assert_eq!(
        values, &want,
        "the linocut's uniform does not hold its parameters"
    );
    assert!(g.source.contains("struct PrintN1 {\n    p_depth: f32,\n    p_gouge: f32,\n    p_seed: f32,\n    p_ink: vec3<f32>,\n    p_cut: f32,\n}"));
    assert!(
        g.source
            .contains("mix(print_n1.p_ink, print_tex(print_n1_in_ground, uv).rgb, cut)"),
        "{}",
        g.source
    );
    assert!(
        g.source.contains("print_frame.px.x * print_frame.res.x"),
        "{}",
        g.source
    );
    assert!(
        !g.source.contains("print_n1_prev"),
        "a one-pass node has no `prev`"
    );
}

negative_control!(
    print_registry_entries_codegen_binds_inputs_and_params,
    "the linocut's ink left at its default",
    expected = "the linocut's uniform does not hold its parameters",
    check_linocut(&generated(
        r#"{"format":"principia.print/1","name":"L","settings":{"s":3},"nodes":{"img":{"type":"render"},
          "cut":{"type":"linocut","params":{"seed":"$s+2","cut":1},"in":{"in":"img"}},
          "out":{"type":"output","in":{"in":"cut"}}},"out":"out"}"#,
    ))
);

/// Codegen refuses what it cannot run: a parameter out of range or not its type's, and nodes reading in a cycle.
fn check_codegen_refusals(cases: &[(&str, &str)]) {
    for (text, why) in cases {
        let graph = Graph::parse(text).expect("parses");
        let e = expand(&graph, &|_: &str| None).expect("expands");
        match generate(&e, &support::registry()) {
            Ok(_) => panic!("a graph codegen cannot run generated: {text}"),
            Err(err) => {
                assert!(err.0.contains(why), "{err} does not say {why:?}");
                assert_eq!(err.to_string(), err.0);
            }
        }
    }
}

const CYCLE: &str = r#"{"format":"principia.print/1","name":"C","nodes":{"a":{"type":"blur","in":{"in":"b"}},
  "b":{"type":"blur","in":{"in":"a"}},"out":{"type":"output","in":{"in":"b"}}},"out":"out"}"#;

const ACYCLIC: &str = r#"{"format":"principia.print/1","name":"A","nodes":{"a":{"type":"blur"},
  "out":{"type":"output","in":{"in":"a"}}},"out":"out"}"#;

#[test]
fn print_registry_entries_codegen_refusals() {
    let with = |params: &str| {
        ACYCLIC.replace(
            r#"{"type":"blur"}"#,
            &format!(r#"{{"type":"blur","params":{params}}}"#),
        )
    };
    check_codegen_refusals(&[
        (
            CYCLE,
            "the nodes [\"a\", \"b\", \"out\"] read each other in a cycle",
        ),
        (
            &with(r#"{"sigma":9}"#),
            "`sigma` = 9 is outside its range [0.5, 8]",
        ),
        (
            &with(r#"{"sigma":0.25}"#),
            "`sigma` = 0.25 is outside its range [0.5, 8]",
        ),
        (&with(r#"{"sgma":1}"#), "`sgma` is no parameter of `blur`"),
        (&with(r#"{"sigma":"x"}"#), "is no value of its kind"),
        (&ACYCLIC.replace("\"blur\"", "\"blurr\""), "not registered"),
    ]);
    // Its range's ends are in it.
    for sigma in ["0.5", "8"] {
        let g = Graph::parse(&with(&format!(r#"{{"sigma":{sigma}}}"#))).expect("parses");
        let e = expand(&g, &|_: &str| None).expect("expands");
        generate(&e, &support::registry()).unwrap_or_else(|e| panic!("sigma {sigma}: {e}"));
    }
}

negative_control!(
    print_registry_entries_codegen_refusals,
    "a graph codegen can run",
    expected = "a graph codegen cannot run generated",
    check_codegen_refusals(&[(ACYCLIC, "")])
);

// ----- parameters named as WGSL reserved words -----

/// An entry whose parameters are named `type`, `smooth`, `from` (as the lab's `ground`, `stereogram` and `unfinished`
/// name theirs), `_` and `__x`, none of them a WGSL identifier.
const RESERVED: &str = r##"{
  "id": "reserved", "label": "Reserved", "category": "photo",
  "doc": "A test entry: parameters named as WGSL reserved words.",
  "inputs": [{ "name": "in", "type": "image", "default": "render" }],
  "outputs": [{ "name": "out", "type": "image" }],
  "params": [
    { "name": "type", "kind": "number", "default": 0, "min": 0, "max": 1, "step": 1, "unit": "none" },
    { "name": "smooth", "kind": "number", "default": 0.5, "min": 0, "max": 1, "step": 0.01, "unit": "fraction" },
    { "name": "from", "kind": "choice", "default": 0, "options": ["a", "b"] },
    { "name": "_", "kind": "colour", "default": "#808080" },
    { "name": "__x", "kind": "number", "default": 1, "min": 0, "max": 2, "step": 1, "unit": "none" }
  ],
  "passes": ["reserved.wgsl"], "cost": "light", "motion": "full", "temporally_stable": true,
  "screen_medium": false, "deviations": ["A test entry."]
}"##;

const RESERVED_PASS: &str = "fn run(uv: vec2<f32>) -> vec4<f32> {
    let k = p_type + p_smooth + p_from + p___x;
    return vec4<f32>(tex(in_in, uv).rgb * p__ * k, 1.0);
}";

/// The reserved entry's module, as `emit` writes it, compiles under naga.
fn check_reserved(emit: fn(String) -> String) {
    let mut registry = support::registry();
    registry
        .register(RESERVED, &|f: &str| {
            (f == "reserved.wgsl").then(|| RESERVED_PASS.to_owned())
        })
        .expect("the reserved entry registers");
    let graph = Graph::parse(
        r#"{"format":"principia.print/1","name":"R","nodes":{"img":{"type":"render"},
          "r":{"type":"reserved","params":{"smooth":0.25},"in":{"in":"img"}},
          "out":{"type":"output","in":{"in":"r"}}},"out":"out"}"#,
    )
    .expect("parses");
    let e = expand(&graph, &|_: &str| None).expect("expands");
    let g = generate(&e, &registry).unwrap_or_else(|e| panic!("does not generate: {e}"));
    assert!(
        g.source.contains("    p_type: f32,") && g.source.contains("print_n1.p_from"),
        "the members are not prefixed: {}",
        g.source
    );
    compile(&emit(g.source));
}

#[test]
fn print_registry_entries_reserved_parameter_names_compile() {
    check_reserved(|s| s);
}

negative_control!(
    print_registry_entries_reserved_parameter_names_compile,
    "the members written by the bare parameter names",
    expected = "the generated WGSL does not parse",
    // The module with each parameter's member and its reads written by the bare parameter name.
    check_reserved(|source| ["type", "smooth", "from", "__x", "_"]
        .iter()
        .fold(source, |s, n| s.replace(&format!("p_{n}"), n)))
);

// ----- a pass's own helpers -----

/// An entry of two passes, each declaring helpers at module level, some of one name in both.
const HELPER: &str = r##"{
  "id": "helper", "label": "Helper", "category": "photo",
  "doc": "A test entry: passes declaring helpers.",
  "inputs": [{ "name": "in", "type": "image", "default": "render" }],
  "outputs": [{ "name": "out", "type": "image" }],
  "params": [],
  "passes": ["helper_a.wgsl", "helper_b.wgsl"], "cost": "light", "motion": "full", "temporally_stable": true,
  "screen_medium": false, "deviations": ["A test entry."]
}"##;

/// The first pass: a `const`, an `override`, an `alias`, a `struct` with a member named as a helper, a `var<private>`
/// and two `fn`s, one named `f`, as a literal's suffix is, a `const` named `x`, as a swizzle is, and a `<` comparing
/// two of them, no template.
const HELPER_A: &str = "// Helpers, some named as the second pass names its own.
const K: f32 = 0.5;
const x: f32 = 2.0;
const B: bool = K < x;
override gain: f32 = 1.0;
alias V = vec3<f32>;
struct S { shade: f32, v: vec2<f32> }
var<private> seen: f32;
fn f(a: f32) -> f32 { return a * 1.0f; }
fn shade(c: V) -> V {
    let s = S(K, vec2<f32>(x, 0.0));
    // Count the shade.
    seen = s.shade + s.v.x;
    /* Again. */ seen = seen + K + s. /* a member */ shade;
    return c * f(s.shade) * gain + seen;
}
fn run(uv: vec2<f32>) -> vec4<f32> {
    return vec4<f32>(shade(tex(in_in, uv).rgb), 1.0);
}";

/// The second pass: a `const`, a `struct` and a `fn` of the first pass's names, a comment ending in `fn` before the
/// first, a local named as a helper, which shadows it, and a local `var`, which keeps its name.
const HELPER_B: &str = "/*fn*/const K: f32 = 0.25;
const W: f32 = 2.0;
struct S { k: f32 }
fn shade(c: vec3<f32>) -> vec3<f32> {
    let W: f32 = S(K).k;
    var t = W;
    return c * t;
}
fn run(uv: vec2<f32>) -> vec4<f32> {
    return vec4<f32>(shade(tex(prev, uv).rgb), 1.0);
}";

/// A graph of two helper nodes in a chain: the module, as `emit` writes it, compiles under naga, each pass's helpers
/// its own, a use after a comment ending in `.`, and a member after a `.` and a comment, among them.
fn check_helpers(emit: fn(String) -> String) {
    let mut registry = support::registry();
    registry
        .register(HELPER, &|f: &str| match f {
            "helper_a.wgsl" => Some(HELPER_A.to_owned()),
            "helper_b.wgsl" => Some(HELPER_B.to_owned()),
            _ => None,
        })
        .expect("the helper entry registers");
    let graph = Graph::parse(
        r#"{"format":"principia.print/1","name":"H","nodes":{"img":{"type":"render"},
          "a":{"type":"helper","in":{"in":"img"}},"b":{"type":"helper","in":{"in":"a"}},
          "out":{"type":"output","in":{"in":"b"}}},"out":"out"}"#,
    )
    .expect("parses");
    let e = expand(&graph, &|_: &str| None).expect("expands");
    let g = generate(&e, &registry).unwrap_or_else(|e| panic!("does not generate: {e}"));
    for k in [1, 2] {
        for needle in [
            format!("fn print_n{k}_pass0_h_shade(c: print_n{k}_pass0_h_V) -> print_n{k}_pass0_h_V {{"),
            format!("let s = print_n{k}_pass0_h_S(print_n{k}_pass0_h_K, vec2<f32>(print_n{k}_pass0_h_x, 0.0));"),
            format!("struct print_n{k}_pass0_h_S {{ shade: f32, v: vec2<f32> }}"),
            format!("// Count the shade.\n    print_n{k}_pass0_h_seen = s.shade + s.v.x;"),
            format!("/* Again. */ print_n{k}_pass0_h_seen = print_n{k}_pass0_h_seen + print_n{k}_pass0_h_K + s. /* a member */ shade;"),
            "return a * 1.0f; }".to_owned(),
            format!("var<private> print_n{k}_pass0_h_seen: f32;"),
            format!("const print_n{k}_pass0_h_B: bool = print_n{k}_pass0_h_K < print_n{k}_pass0_h_x;\noverride print_n{k}_pass0_h_gain"),
            format!("/*fn*/const print_n{k}_pass1_h_K: f32 = 0.25;"),
            format!("fn print_n{k}_pass1_h_shade(c: vec3<f32>) -> vec3<f32> {{"),
            format!("    let print_n{k}_pass1_h_W: f32 = print_n{k}_pass1_h_S(print_n{k}_pass1_h_K).k;\n    var t = print_n{k}_pass1_h_W;\n    return c * t;"),
        ] {
            assert!(g.source.contains(&needle), "the helpers are not the pass's own: no {needle} in {}", g.source);
        }
    }
    compile(&emit(g.source));
}

#[test]
fn print_registry_entries_pass_helpers_compile() {
    check_helpers(|s| s);
}

negative_control!(
    print_registry_entries_pass_helpers_compile,
    "the helpers declared by their bare names, as the pass writes them",
    expected = "the generated WGSL does not parse",
    check_helpers(|source| ["n1_pass0", "n1_pass1", "n2_pass0", "n2_pass1"]
        .iter()
        .fold(source, |s, p| s.replace(&format!("print_{p}_h_"), "")))
);

// ----- nested block comments -----

/// An entry of one pass, `nested.wgsl`.
const NESTED: &str = r##"{
  "id": "nested", "label": "Nested", "category": "photo",
  "doc": "A test entry: a pass with nested block comments.",
  "inputs": [{ "name": "in", "type": "image", "default": "render" }],
  "outputs": [{ "name": "out", "type": "image" }],
  "params": [],
  "passes": ["nested.wgsl"], "cost": "light", "motion": "full", "temporally_stable": true,
  "screen_medium": false, "deviations": ["A test entry."]
}"##;

/// A pass whose block comments nest, as WGSL's do, each holding text that is code once the comment ends at its first
/// `*/`: a retired `fn max`, an open brace before a helper's declaration, and a member after a `.`.
const NESTED_PASS: &str = "const K: f32 = 0.5;
struct S { shade: f32 }
fn shade(c: f32) -> f32 { return c * K; }
/* retired: /* v1 */ fn max(a: f32, b: f32) -> f32 { return a; } */
/* an open brace: /* v1 */ { */
fn lift(c: f32) -> f32 { return max(c, K); }
fn run(uv: vec2<f32>) -> vec4<f32> {
    let s = S(shade(uv.x));
    return vec4<f32>(lift(s. /* see /* v1 */ notes */ shade), 0.0, 0.0, 1.0);
}";

/// The generated module of one node of the nested entry, its pass `pass`.
fn nested(pass: &'static str) -> Result<Generated, String> {
    let mut registry = support::registry();
    registry
        .register(NESTED, &|f: &str| {
            (f == "nested.wgsl").then(|| pass.to_owned())
        })
        .expect("the nested entry registers");
    let graph = Graph::parse(
        r#"{"format":"principia.print/1","name":"N","nodes":{"img":{"type":"render"},
          "n":{"type":"nested","in":{"in":"img"}},"out":{"type":"output","in":{"in":"n"}}},"out":"out"}"#,
    )
    .expect("parses");
    let e = expand(&graph, &|_: &str| None).expect("expands");
    generate(&e, &registry).map_err(|e| e.0)
}

/// The nested pass's module, as `emit` writes it, skips each block comment through the `*/` closing it: the member
/// keeps its name, the builtin `max` is not the pass's, the helper after the open brace is, and it compiles.
fn check_nested(emit: fn(String) -> String) {
    let g = nested(NESTED_PASS).unwrap_or_else(|e| panic!("does not generate: {e}"));
    let source = emit(g.source);
    let missing: Vec<&str> = [
        "/* retired: /* v1 */ fn max(a: f32, b: f32) -> f32 { return a; } */\n/* an open brace: /* v1 */ { */\n",
        "fn print_n1_pass0_h_lift(c: f32) -> f32 {",
        "return max(c, print_n1_pass0_h_K); }",
        "(s. /* see /* v1 */ notes */ shade)",
    ]
    .into_iter()
    .filter(|needle| !source.contains(needle))
    .collect();
    assert!(
        missing.is_empty(),
        "a nested comment ends at its first `*/`: no {missing:?} in {source}"
    );
    compile(&source);
}

#[test]
fn print_registry_entries_nested_comments_skipped() {
    check_nested(|s| s);
}

negative_control!(
    print_registry_entries_nested_comments_skipped,
    "the helper after the open brace left unrenamed, as a scan ending the comment at its first `*/` leaves it",
    expected = "a nested comment ends at its first `*/`",
    check_nested(|s| s.replace("print_n1_pass0_h_lift", "lift"))
);

/// A pass whose block comment, `/* /* v1 */ open`, is never closed is refused, the refusal naming it.
fn check_unclosed(pass: &'static str) {
    match nested(pass) {
        Ok(g) => panic!("a pass with an unclosed comment generated: {}", g.source),
        Err(e) => assert_eq!(
            e,
            "node `n`'s pass `nested.wgsl` has an unclosed `/*` comment"
        ),
    }
}

#[test]
fn print_registry_entries_unclosed_comment_refused() {
    check_unclosed(
        "fn run(uv: vec2<f32>) -> vec4<f32> { return vec4<f32>(uv, 0.0, 1.0); }\n/* /* v1 */ open",
    );
}

negative_control!(
    print_registry_entries_unclosed_comment_refused,
    "the comment closed",
    expected = "a pass with an unclosed comment generated",
    check_unclosed(
        "fn run(uv: vec2<f32>) -> vec4<f32> { return vec4<f32>(uv, 0.0, 1.0); }\n/* /* v1 */ open */"
    )
);

// ----- registration refusals -----

/// The output entry's declaration with `member` taken out.
fn without(member: &str) -> String {
    let mut json: serde_json::Value = serde_json::from_str(NODES[0].1).expect("JSON");
    json.as_object_mut().expect("an object").remove(member);
    json.to_string()
}

/// The declaration `decl` fails to register, the refusal naming `member`.
fn check_unregistered(decl: &str, member: &str) {
    match Registry::new().register(decl, &support::wgsl) {
        Ok(e) => panic!("the entry `{}` registered without `{member}`", e.id),
        Err(e) => assert!(
            e.0.contains(&format!("`{member}`")),
            "{e} does not name `{member}`"
        ),
    }
}

#[test]
fn print_registry_entries_without_motion_policy_refused() {
    assert_eq!(NODES[0].0, "output.json");
    check_unregistered(&without("motion"), "motion");
}

negative_control!(
    print_registry_entries_without_motion_policy_refused,
    "the entry with its motion policy",
    expected = "registered without `motion`",
    check_unregistered(NODES[0].1, "motion")
);

#[test]
fn print_registry_entries_without_temporally_stable_refused() {
    check_unregistered(&without("temporally_stable"), "temporally_stable");
}

negative_control!(
    print_registry_entries_without_temporally_stable_refused,
    "the entry with `temporally_stable`",
    expected = "registered without `temporally_stable`",
    check_unregistered(NODES[0].1, "temporally_stable")
);

// ----- the stain's codegen never sees a Print graph node -----

/// The stain's codegen, given `function` as a colour occupant's node, refuses it as a Print graph node's.
fn check_stain_refuses(function: &str) {
    let node = StainNode {
        id: 0,
        output: ValueType::Vec3,
        call: Call {
            name: "output".into(),
            function: function.into(),
            args: vec![Arg::Ctx],
            params: vec![],
        },
        inputs: vec![],
    };
    match stain_generate(Slot::Colour, &node) {
        Ok(_) => panic!("the stain's codegen took the node `{function}`"),
        Err(e) => assert!(e.0.contains("Print graph"), "{e}"),
    }
}

#[test]
fn print_registry_entries_stain_codegen_refuses_print_node() {
    let plain = generated(&shipped().presets["Plain"].canonical().expect("canonical"));
    for pass in &plain.passes {
        assert!(pass.function.starts_with(PREFIX) && pass.entry_point.starts_with(PREFIX));
        check_stain_refuses(&pass.function);
        check_stain_refuses(&pass.entry_point);
    }
    check_stain_refuses("print_tex");
}

negative_control!(
    print_registry_entries_stain_codegen_refuses_print_node,
    "a stain occupant's own library function",
    expected = "the stain's codegen took the node `oklch_to_linear`",
    check_stain_refuses("oklch_to_linear")
);
