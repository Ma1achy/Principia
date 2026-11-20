//! The occupant tree's codegen and the map parameters' schema (TASK-M7-03; colour_composition §1, §5, §8):
//! - REQ-GEN-022: the same tree generates byte-identical WGSL, which the one assembler compiles against the shared
//!   library; a node's function keeps its name and text when another node is edited; each function's header comment
//!   carries its node's name and parameter values; and a slider edit writes the node's uniform with the pipeline-compile
//!   counter unchanged (`codegen_deterministic_*`; the last on the GPU harness's device);
//! - REQ-COL-031: the schema clamps each of L, C, κ, f, N, ks and s to its adopted range, and the range reaches the
//!   generated uniform's declaration (`param_schema_ranges_*`).
//!
//! The trees are the tests' own, each node a call into the shared library: the algebra's operations are TASK-M7-05 to
//! -10's. Each test registers its negative control (R-176).

use render::assemble::{
    self, Declaration, Kind, Node as StainNode, Occupant, Stain, Tier, UniformType,
};
use render::codegen::schema::MapParam;
use render::codegen::{
    function_name, generate, uniform_name, Arg, Call, Generated, Node, Param, Slot, ValueType,
};
use render::compositor::LAYER_FORMAT;
use render::frame_record::{Applied, RenderLoop};
use render::pipeline_cache::Requested;
use validation::gpu::GpuHarness;
use validation::negative_control;

// ----- the tests' trees -----

fn node(id: u32, output: ValueType, call: Call, inputs: Vec<Node>) -> Node {
    Node {
        id,
        output,
        call,
        inputs,
    }
}

fn call(name: &str, function: &str, args: Vec<Arg>, params: Vec<Param>) -> Call {
    Call {
        name: name.into(),
        function: function.into(),
        args,
        params,
    }
}

/// An `f32` leaf: the lightness `L`, `f32(L)`.
fn lightness(id: u32, l: f64) -> Node {
    let c = call(
        "lightness",
        "f32",
        vec![Arg::Param(0)],
        vec![Param::ranged("L", MapParam::L, l)],
    );
    node(id, ValueType::F32, c, vec![])
}

/// `f32 → vec3`: the greyscale ramp.
fn grey(id: u32, input: Node) -> Node {
    let c = call("grey", "ramp_grey", vec![Arg::Input(0)], vec![]);
    node(id, ValueType::Vec3, c, vec![input])
}

/// A `vec3` leaf: OKLCH at `L`, `C` and a hue in turns.
fn lch(id: u32, l: f64, c: f64, hue: f64) -> Node {
    let k = call(
        "oklch",
        "oklch_to_linear",
        vec![Arg::Param(0), Arg::Param(1), Arg::Param(2)],
        vec![
            Param::ranged("L", MapParam::L, l),
            Param::ranged("C", MapParam::C, c),
            Param::free("hue", UniformType::F32, vec![hue]),
        ],
    );
    node(id, ValueType::Vec3, k, vec![])
}

/// `vec3, vec3 → vec3`: a constant-weight blend at strength `s`.
fn mix(id: u32, a: Node, b: Node, s: f64) -> Node {
    let c = call(
        "mix_const",
        "mix",
        vec![Arg::Input(0), Arg::Input(1), Arg::Param(0)],
        vec![Param::ranged("s", MapParam::S, s)],
    );
    node(id, ValueType::Vec3, c, vec![a, b])
}

/// `mix_const(oklch, grey(lightness))`, ids 3 over 0 and 2 over 1, its lightness `l`.
fn tree(l: f64) -> Node {
    mix(3, lch(0, 0.7, 0.12, 0.3), grey(2, lightness(1, l)), 0.25)
}

fn generated(root: &Node) -> Generated {
    generate(Slot::Colour, root).unwrap_or_else(|e| panic!("{e}"))
}

/// The source of `ftle` (0) into the colour `colour` (1), the pass-through combiner (2) and OUT (3).
fn stain(colour: &str) -> Stain {
    let n = |kind, occupant, inputs: &[Option<usize>]| StainNode {
        kind,
        occupant,
        inputs: inputs.to_vec(),
    };
    Stain::new(vec![
        n(Kind::Source, Occupant::Field("ftle".into()), &[]),
        n(Kind::Colour, Occupant::Custom(colour.into()), &[Some(0)]),
        n(
            Kind::Combiner,
            Occupant::BuiltIn("pass_through".into()),
            &[Some(1), None],
        ),
        n(Kind::Out, Occupant::None, &[Some(2)]),
    ])
    .unwrap_or_else(|e| panic!("{e}"))
}

// ----- REQ-GEN-022: deterministic, and one compile path -----

/// Two generations of one tree, each built afresh, are byte-identical, and the text assembles through the one
/// assembler, every node's function there under its stain node's prefix.
fn check_deterministic(first: &Node, second: &Node) {
    let (a, b) = (generated(first), generated(second));
    assert!(
        a.source == b.source,
        "the same graph's WGSL is not byte-identical:\n{}\n---\n{}",
        a.source,
        b.source
    );
    let fragment = assemble::assemble(&stain(&a.source), Tier::FULL)
        .unwrap_or_else(|e| panic!("the generated occupant did not assemble: {e}"));
    for f in &a.functions {
        assert!(
            fragment
                .source
                .contains(&format!("fn n1_{}(ctx: Ctx)", f.name)),
            "{} is not in the assembled source",
            f.name
        );
    }
}

#[test]
fn codegen_deterministic_same_graph_same_bytes() {
    check_deterministic(&tree(0.6), &tree(0.6));
}

negative_control!(
    codegen_deterministic_same_graph_same_bytes,
    "another lightness is another text",
    expected = "is not byte-identical",
    check_deterministic(&tree(0.6), &tree(0.5))
);

/// The functions of the nodes `edit` leaves alone keep their names and their text, byte for byte.
fn check_names_survive(edit: impl Fn(Node) -> Node) {
    let before = generated(&tree(0.6));
    let after = generated(&edit(tree(0.6)));
    for id in [1, 2, 3] {
        let find = |g: &Generated| g.functions.iter().find(|f| f.id == id).cloned();
        let (b, a) = (find(&before), find(&after));
        assert!(
            b.is_some() && b == a,
            "node {id}'s function did not survive the edit: {b:?} → {a:?}"
        );
        assert_eq!(b.map(|f| f.name), Some(function_name(id)));
    }
}

#[test]
fn codegen_deterministic_names_survive_unrelated_edit() {
    // Node 0's operation is replaced: an OKLCH leaf becomes a greyscale of a new lightness, node 5.
    check_names_survive(|mut t| {
        t.inputs[0] = grey(0, lightness(5, 0.4));
        t
    });
}

negative_control!(
    codegen_deterministic_names_survive_unrelated_edit,
    "renumbering node 2 renames its function",
    expected = "node 2's function did not survive the edit",
    check_names_survive(|mut t| {
        t.inputs[1].id = 7;
        t
    })
);

/// Each function's first line is its header comment: `// node_<id>: <name>`, then its parameters' values.
fn check_comments(want: &[(u32, &str)]) {
    let g = generated(&tree(0.95));
    for &(id, line) in want {
        let f = g
            .functions
            .iter()
            .find(|f| f.id == id)
            .unwrap_or_else(|| panic!("no function for node {id}"));
        assert_eq!(
            f.text.lines().next(),
            Some(line),
            "node {id}'s function does not carry its name and params comment"
        );
    }
}

/// Every node's header comment, the lightness clamped to L's range.
const COMMENTS: [(u32, &str); 4] = [
    (0, "// node_0: oklch — L = 0.7, C = 0.12, hue = 0.3"),
    (1, "// node_1: lightness — L = 0.9"),
    (2, "// node_2: grey"),
    (3, "// node_3: mix_const — s = 0.25"),
];

#[test]
fn codegen_deterministic_comment_carries_name_and_params() {
    check_comments(&COMMENTS);
    let g = generated(&tree(0.6));
    assert!(g.functions[1].text.starts_with(
        "// node_1: lightness — L = 0.6\n\
         // @uniform node_1_L: f32 = 0.6 [0.35, 0.9]\n\
         fn node_1(ctx: Ctx) -> f32 {\n    return f32(uniforms.node_1_L);\n}\n"
    ));
}

negative_control!(
    codegen_deterministic_comment_carries_name_and_params,
    "the lightness as given, unclamped, is not the comment's value",
    expected = "node 1's function does not carry its name and params comment",
    check_comments(&[(1, "// node_1: lightness — L = 0.95")])
);

/// A vector parameter is declared and commented as `(a, b, c)`, and the brightness slot's function returns its `f32`
/// root: `want` is the generated text, whole.
fn check_vector_and_brightness(rgb: Vec<f64>, want: &str) {
    let swatch = node(
        0,
        ValueType::Vec3,
        call(
            "swatch",
            "vec3f",
            vec![Arg::Param(0)],
            vec![Param::free("rgb", UniformType::Vec3, rgb)],
        ),
        vec![],
    );
    let luma = call(
        "luma",
        "dot",
        vec![Arg::Input(0), Arg::Param(0)],
        vec![Param::free("w", UniformType::Vec3, vec![0.25, 0.5, 0.25])],
    );
    let root = node(1, ValueType::F32, luma, vec![swatch]);
    let g = generate(Slot::Brightness, &root).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(g.source, want, "the generated text is not the expected one");
    let d = Declaration::parse(&g.source).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(d.uniforms[0].default, vec![1.0, 0.5, 0.25]);
    let n = |kind, occupant, inputs: &[Option<usize>]| StainNode {
        kind,
        occupant,
        inputs: inputs.to_vec(),
    };
    let s = Stain::new(vec![
        n(Kind::Source, Occupant::Field("ftle".into()), &[]),
        n(Kind::Brightness, Occupant::Custom(g.source), &[Some(0)]),
        n(
            Kind::Combiner,
            Occupant::BuiltIn("pass_through".into()),
            &[None, Some(1)],
        ),
        n(Kind::Out, Occupant::None, &[Some(2)]),
    ])
    .unwrap_or_else(|e| panic!("{e}"));
    assemble::assemble(&s, Tier::FULL)
        .unwrap_or_else(|e| panic!("the generated brightness did not assemble: {e}"));
}

/// [`check_vector_and_brightness`]'s text.
const VECTOR_AND_BRIGHTNESS: &str = "\
// A brightness occupant generated from its expression tree (colour_composition §1, §5): one function per node, each
// node's inputs before it, then the slot's function, which returns the root's.

// node_0: swatch — rgb = (1, 0.5, 0.25)
// @uniform node_0_rgb: vec3<f32> = (1, 0.5, 0.25)
fn node_0(ctx: Ctx) -> vec3<f32> {
    return vec3f(uniforms.node_0_rgb);
}

// node_1: luma — w = (0.25, 0.5, 0.25)
// @uniform node_1_w: vec3<f32> = (0.25, 0.5, 0.25)
fn node_1(ctx: Ctx) -> f32 {
    return dot(node_0(ctx), uniforms.node_1_w);
}

// The brightness slot: the root, node_1.
fn brightness(ctx: Ctx) -> f32 {
    return node_1(ctx);
}
";

#[test]
fn codegen_deterministic_vector_params_and_brightness_slot() {
    check_vector_and_brightness(vec![1.0, 0.5, 0.25], VECTOR_AND_BRIGHTNESS);
}

negative_control!(
    codegen_deterministic_vector_params_and_brightness_slot,
    "another swatch is another text",
    expected = "the generated text is not the expected one",
    check_vector_and_brightness(vec![1.0, 0.5, 0.5], VECTOR_AND_BRIGHTNESS)
);

/// A slider edit on node 1's lightness, past L's range: the schema clamps it and it is written to the compiled
/// stain's uniform, compiling nothing and keeping the key. With `regenerate`, the edit regenerates the occupant instead,
/// as a slider must not.
fn check_slider(regenerate: bool) {
    let h = GpuHarness::new().expect("a GPU device");
    let mut rl = RenderLoop::new(h.device(), h.queue(), LAYER_FORMAT, 4, 2)
        .unwrap_or_else(|e| panic!("{e}"));
    let keys = [0, 1, 2, 3];
    let first = stain(&generated(&tree(0.6)).source);
    rl.cache()
        .request(&first, &keys, Tier::FULL)
        .unwrap_or_else(|e| panic!("{e}"));
    rl.cache().wait();
    assert!(rl.cache().errors().is_empty(), "{:?}", rl.cache().errors());
    let key = rl.cache().current().expect("compiled").key();
    assert_eq!(rl.cache().compiles(), 1);

    let raw = 0.95;
    let value = MapParam::L.clamp(raw).expect("a number");
    let name = uniform_name(1, "L");
    if regenerate {
        let second = stain(&generated(&tree(value)).source);
        rl.cache()
            .request(&second, &keys, Tier::FULL)
            .unwrap_or_else(|e| panic!("{e}"));
        rl.cache().wait();
    } else {
        assert!(
            rl.set_param(1, &name, &[raw]).is_err(),
            "the uniform took a value past L's range"
        );
        assert_eq!(
            rl.set_param(1, &name, &[value])
                .unwrap_or_else(|e| panic!("{e}")),
            Applied::Now,
            "the slider did not write the live node's uniform"
        );
        assert_eq!(
            rl.cache()
                .request(&first, &keys, Tier::FULL)
                .unwrap_or_else(|e| panic!("{e}")),
            Requested::Hit
        );
    }
    assert_eq!(rl.cache().compiles(), 1, "a slider change recompiled");
    assert_eq!(rl.cache().current().expect("current").key(), key);
}

#[test]
fn codegen_deterministic_slider_rebinds_without_recompile() {
    check_slider(false);
}

negative_control!(
    codegen_deterministic_slider_rebinds_without_recompile,
    "regenerating the occupant on a slider edit compiles again",
    expected = "a slider change recompiled",
    check_slider(true)
);

/// `root` is refused, and why.
fn check_refused(root: &Node, why: &str) {
    match generate(Slot::Colour, root) {
        Ok(_) => panic!("the tree generated; it must be refused for `{why}`"),
        Err(e) => assert!(
            e.to_string().contains(why),
            "refused for `{e}`, not `{why}`"
        ),
    }
}

#[test]
fn codegen_deterministic_refuses_malformed_trees() {
    check_refused(
        &mix(3, lch(0, 0.7, 0.1, 0.0), grey(0, lightness(1, 0.6)), 0.5),
        "node 0: the id is used twice",
    );
    for name in ["oklch to linear", "", "_", "__f", "1f", "f-g", "é"] {
        let mut bad = lch(0, 0.7, 0.1, 0.0);
        bad.call.function = name.into();
        check_refused(&bad, &format!("`{name}` is no function name"));
    }
    for name in ["_f", "f_1", "F"] {
        let mut ok = lch(0, 0.7, 0.1, 0.0);
        ok.call.function = name.into();
        assert!(generate(Slot::Colour, &ok).is_ok(), "`{name}` is a name");
    }
    let mut bad = lch(0, 0.7, 0.1, 0.0);
    bad.call.params[2].name = "C".into();
    check_refused(
        &bad,
        "the parameter name `C` is no identifier, or is used twice",
    );
    let mut bad = lch(0, 0.7, 0.1, 0.0);
    bad.call.name = "oklch\nfn x".into();
    check_refused(&bad, "holds a control character");
    let mut bad = grey(1, lightness(0, 0.5));
    bad.call.args = vec![Arg::Input(1)];
    check_refused(&bad, "no input 1");
    let mut bad = grey(1, lightness(0, 0.5));
    bad.call.args = vec![Arg::Param(0)];
    check_refused(&bad, "no parameter 0");
    check_refused(
        &lightness(0, f64::NAN),
        "parameter `L` = [NaN] is no value of its uniform",
    );
    let mut bad = lightness(0, 0.5);
    bad.call.params[0].value = vec![0.5, 0.5];
    check_refused(
        &bad,
        "parameter `L` = [0.5, 0.5] is no value of its uniform",
    );
    let mut bad = lightness(0, 0.5);
    bad.call.params[0].ty = UniformType::U32;
    check_refused(&bad, "parameter `L` = [0.5] is no value of its uniform");
    let mut bad = lch(0, 0.7, 0.1, 0.0);
    bad.call.params[2].value = vec![0.0, 1.0];
    check_refused(
        &bad,
        "parameter `hue` = [0.0, 1.0] is no value of its uniform",
    );
}

negative_control!(
    codegen_deterministic_refuses_malformed_trees,
    "a well-formed tree generates",
    expected = "the tree generated",
    check_refused(&tree(0.6), "the id is used twice")
);

// ----- REQ-COL-031: the adopted ranges -----

/// colour_composition §8's ranges, as written there.
const ADOPTED: [(MapParam, &str, f64, f64); 7] = [
    (MapParam::L, "L", 0.35, 0.90),
    (MapParam::C, "C", 0.05, 0.22),
    (MapParam::Kappa, "κ", 0.5, 12.0),
    (MapParam::F, "f", 2.0, 14.0),
    (MapParam::N, "N", 12.0, 96.0),
    (MapParam::Ks, "ks", 1.0, 20.0),
    (MapParam::S, "s", 0.0, 1.0),
];

/// Each parameter's range is `table`'s, and the schema clamps into it: below to `lo`, above to `hi`, inside
/// unchanged, ±∞ to the ends; NaN has no clamp.
fn check_ranges(table: &[(MapParam, &str, f64, f64)]) {
    assert_eq!(
        table.iter().map(|r| r.0).collect::<Vec<_>>(),
        MapParam::ALL,
        "the table is not every parameter"
    );
    for &(p, symbol, lo, hi) in table {
        assert_eq!(p.symbol(), symbol);
        assert_eq!(
            p.range(),
            (lo, hi),
            "{symbol}'s range is not the adopted range"
        );
        let mid = 0.5 * (lo + hi);
        for (v, want) in [
            (lo - 1.0, lo),
            (lo - 1e-9, lo),
            (lo, lo),
            (mid, mid),
            (hi, hi),
            (hi + 1e-9, hi),
            (hi + 1.0, hi),
            (f64::NEG_INFINITY, lo),
            (f64::INFINITY, hi),
        ] {
            assert_eq!(
                p.clamp(v),
                Some(want),
                "{symbol} clamps {v} to {:?}",
                p.clamp(v)
            );
        }
        assert_eq!(p.clamp(f64::NAN), None, "{symbol} clamps NaN");
    }
}

#[test]
fn param_schema_ranges_clamp_each() {
    check_ranges(&ADOPTED);
}

negative_control!(
    param_schema_ranges_clamp_each,
    "κ to 10 is not the adopted range",
    expected = "κ's range is not the adopted range",
    check_ranges(&{
        let mut t = ADOPTED;
        t[2].3 = 10.0;
        t
    })
);

/// A map parameter in a generated node, its value `raw`: the declaration's default is `want(lo, hi)`, and it carries the
/// adopted range, which the assembler parses and holds every value to.
fn check_declared(raw: f64, want: fn(f64, f64) -> f64) {
    for &(p, symbol, lo, hi) in &ADOPTED {
        let c = call(
            "p",
            "f32",
            vec![Arg::Param(0)],
            vec![Param::ranged("v", p, raw)],
        );
        let g = generated(&grey(1, node(0, ValueType::F32, c, vec![])));
        let d = Declaration::parse(&g.source).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(d.uniforms.len(), 1);
        let u = &d.uniforms[0];
        assert_eq!(u.name, "node_0_v");
        assert_eq!(u.range, Some((lo, hi)), "{symbol}'s declared range");
        assert_eq!(
            u.default,
            vec![want(lo, hi)],
            "{symbol}'s declared value is not clamped"
        );
        assert!(u.admits(&[lo]) && u.admits(&[hi]));
        assert!(
            !u.admits(&[lo - 1e-6]) && !u.admits(&[hi + 1e-6]),
            "{symbol}'s uniform admits values past its range"
        );
    }
}

#[test]
fn param_schema_ranges_reach_the_declaration() {
    check_declared(1e3, |_, hi| hi);
    check_declared(-1e3, |lo, _| lo);
    check_declared(f64::INFINITY, |_, hi| hi);
}

negative_control!(
    param_schema_ranges_reach_the_declaration,
    "the raw value, unclamped, is not the declaration's",
    expected = "declared value is not clamped",
    check_declared(1e3, |_, _| 1e3)
);
