//! A custom node that goes wrong falls back alone, says so at the author's line, and a generated node ejected and
//! reverted is as generated, byte for byte. Pass criteria, written before the first run:
//!
//! 1. Colour node `A` holds a valid source, post node `B` holds one, and both are last valid. Then `A` is edited to
//!    WGSL naga refuses and `B` to another valid source, in one request. Settled: exactly `A` failed; the stain that
//!    renders has `A` at its last valid source and `B` at its new one, and assembles to the text of that stain
//!    exactly; Problems holds one entry, against `A`, `Code`, at the line of the broken statement in `A`'s own
//!    source (not the assembled module's); `A` is `Failed` and `B` `Valid`.
//! 2. A declaration edit that does not parse leaves the node its last valid description, and Problems says which
//!    declaration line.
//! 3. Eject: every node ejected and reverted is the generated source byte for byte; one node edited leaves every
//!    peer's function byte for byte; a regenerated tree reports an ejected node gone.
//! 4. Values follow a declaration: a value outside a new range is clamped and said so, a changed type takes the
//!    default, a removed uniform's value comes back with it.
//!
//! Each check has a control arm, a run with the property broken, which must fail the check.

use std::collections::{BTreeMap, HashMap};

use render::assemble::{self, Kind, Node, Occupant, Stain, Tier, Uniform, UniformType};
use render::codegen::eject::{Drift, Ejected};
use render::codegen::{generate, Arg, Call, Node as Tree, Param, Slot, ValueType};
use render::custom::{
    line_in, reconcile, settle, template, CustomNodes, Note, Origin, Remembered, Status,
};
#[cfg(feature = "controls")]
use render::custom::{probe, Fault as CustomFault};
use render::pipeline_cache::{CompileError, NodeKey};
use render::registry::ZERO;
use validation::negative_control;

const A_KEY: NodeKey = 10;
const B_KEY: NodeKey = 11;

const A_V1: &str = "fn colour(ctx: Ctx) -> vec3<f32> {\n    return vec3<f32>(0.25);\n}\n";
const A_BROKEN: &str =
    "fn colour(ctx: Ctx) -> vec3<f32> {\n    let k = 0.5;\n    return vec3<f32>(k 0.5, 0.5);\n}\n";
const B_V1: &str = "fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb * 0.5; }";
const B_V2: &str = "fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb * 0.75; }";

fn node(kind: Kind, occupant: Occupant, inputs: &[Option<usize>]) -> Node {
    Node {
        kind,
        occupant,
        inputs: inputs.to_vec(),
    }
}

/// zero source → colour `A` → pass-through combiner → post `B` → OUT.
fn graph(a: &str, b: &str) -> Vec<Node> {
    vec![
        node(Kind::Source, Occupant::BuiltIn(ZERO.to_owned()), &[]),
        node(Kind::Colour, Occupant::Custom(a.to_owned()), &[Some(0)]),
        node(
            Kind::Combiner,
            Occupant::BuiltIn("pass_through".into()),
            &[Some(1), None],
        ),
        node(Kind::Post, Occupant::Custom(b.to_owned()), &[Some(2)]),
        node(Kind::Out, Occupant::None, &[Some(3)]),
    ]
}

const KEYS: [NodeKey; 5] = [1, A_KEY, 2, B_KEY, 3];

fn stain(g: Vec<Node>) -> Stain {
    Stain::new(g).unwrap_or_else(|e| panic!("{e}"))
}

#[derive(Clone, Copy, PartialEq)]
#[cfg_attr(not(feature = "controls"), allow(dead_code))]
enum Fault {
    /// No last valid occupant is kept.
    NoLastValid,
}

fn check_fallback(fault: Option<Fault>) {
    let mut last_valid: HashMap<NodeKey, Occupant> = HashMap::new();
    if fault != Some(Fault::NoLastValid) {
        for (n, k) in graph(A_V1, B_V1).iter().zip(KEYS) {
            last_valid.insert(k, n.occupant.clone());
        }
    }
    let asked = stain(graph(A_BROKEN, B_V2));
    let settled = match settle(&asked, &KEYS, Tier::FULL, &last_valid) {
        Ok(s) => s,
        Err(e) => panic!("the stain must still render, `A` at its last valid source: {e}"),
    };
    let blamed: Vec<usize> = settled.failed.iter().map(|(i, _)| *i).collect();
    assert_eq!(blamed, vec![1], "exactly the colour node failed");
    let want = graph(A_V1, B_V2);
    let rendering = settled.stain.nodes();
    let got: Vec<&Occupant> = rendering.iter().map(|n| &n.occupant).collect();
    let want_occupants: Vec<&Occupant> = want.iter().map(|n| &n.occupant).collect();
    assert_eq!(
        got, want_occupants,
        "`A` at its last valid source, `B` at its new one"
    );
    let reference =
        assemble::assemble(&stain(graph(A_V1, B_V2)), Tier::FULL).unwrap_or_else(|e| panic!("{e}"));
    assert!(
        settled.fragment.source == reference.source,
        "the fallback assembles to the stain `A` at its last valid source and `B` at its new one is"
    );

    let errors: Vec<CompileError> = settled
        .failed
        .iter()
        .map(|(i, e)| CompileError::Node {
            node: *i,
            key: KEYS[*i],
            error: e.clone(),
        })
        .collect();
    let mut custom = CustomNodes::new();
    custom.offer(A_KEY, Kind::Colour, A_BROKEN);
    custom.offer(B_KEY, Kind::Post, B_V2);
    custom.answered(asked.nodes(), &KEYS, &errors);
    let problems = custom.problems(&errors);
    assert_eq!(problems.0.len(), 1, "one problem: {problems:?}");
    let p = &problems.0[0];
    assert_eq!(p.node, Some(A_KEY), "against the node that failed");
    assert_eq!(p.origin, Origin::Code);
    assert_eq!(
        p.line,
        Some(3),
        "at the author's own line of `A`'s source: {}",
        p.message
    );
    assert_eq!(custom.status(A_KEY), Some(Status::Failed));
    assert_eq!(custom.status(B_KEY), Some(Status::Valid));
}

/// The `wgsl:<line>` of a naga report: the assembled module's line, as a finder that takes it for the author's.
#[cfg(feature = "controls")]
fn assembled_line(report: &str) -> Option<usize> {
    let at = report.find("wgsl:")? + 5;
    report[at..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .ok()
}

#[test]
fn custom_node_fallback_isolates_the_failing_node() {
    check_fallback(None);
}

negative_control!(
    custom_node_fallback_isolates_the_failing_node,
    "no last valid occupant kept",
    expected = "the stain must still render",
    check_fallback(Some(Fault::NoLastValid)),
);

#[test]
fn custom_node_fallback_reverts_only_the_failing_node() {
    check_fallback(None);
}

negative_control!(
    custom_node_fallback_reverts_only_the_failing_node,
    "settle reverts the valid edit too",
    expected = "`A` at its last valid source, `B` at its new one",
    probe::with(CustomFault::RevertsEverything, || check_fallback(None)),
);

#[test]
fn custom_node_fallback_names_the_authors_line() {
    check_fallback(None);
    check_line(line_in);
}

negative_control!(
    custom_node_fallback_names_the_authors_line,
    "problem_of reports the assembled module's line",
    expected = "at the author's own line",
    probe::with(CustomFault::AssembledLine, || check_fallback(None)),
);

/// A quoted line the node's text does not have (it is the prelude's) gives no line, not a wrong one; one it has, its own.
fn check_line(find: fn(&str, &str) -> Option<usize>) {
    let prelude = "error\n  ┌─ wgsl:900:5\n  │\n900 │ let not_in_the_node = 1;\n  │ ^";
    assert_eq!(
        find(A_V1, prelude),
        None,
        "a line of the prelude is no line of the node"
    );
    let own = "error\n  ┌─ wgsl:900:5\n  │\n900 │     return vec3<f32>(0.25);\n  │ ^";
    assert_eq!(
        find(A_V1, own),
        Some(2),
        "a line of the node is found at the author's number"
    );
}

#[test]
fn custom_node_fallback_line_is_the_authors_not_the_preludes() {
    check_line(line_in);
}

negative_control!(
    custom_node_fallback_line_is_the_authors_not_the_preludes,
    "the report's own number taken as the line",
    expected = "a line of the prelude is no line of the node",
    check_line(|_, report| assembled_line(report)),
);

fn check_declaration_edit(keep: bool) {
    let good = "// @name Mine\n// @uniform k: f32 = 1 [0, 2]\n// @control k slider \"K\"\nfn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(uniforms.k); }\n";
    let half = "// @name Mine\n// @uniform k: f32 = 1 [0, 2]\n// @control k slidr \"K\"\nfn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(uniforms.k); }\n";
    let mut custom = CustomNodes::new();
    custom.offer(A_KEY, Kind::Colour, good);
    if keep {
        custom.offer(A_KEY, Kind::Colour, half);
    } else {
        // The broken half-state replaces the description, as a node that keeps nothing would.
        custom.forget(A_KEY);
        custom.offer(A_KEY, Kind::Colour, half);
    }
    let d = custom
        .description(A_KEY)
        .expect("the node keeps its last valid description while a declaration is half typed");
    assert_eq!(d.uniforms.len(), 1, "its controls stay");
    let problems = custom.problems(&[]);
    let p = problems.of(A_KEY).next().expect("a problem for the node");
    assert_eq!((p.origin, p.line), (Origin::Declaration, Some(3)));
    assert_eq!(custom.last_valid_text(A_KEY), Some(good));
}

#[test]
fn custom_node_fallback_keeps_the_last_valid_description() {
    check_declaration_edit(true);
}

negative_control!(
    custom_node_fallback_keeps_the_last_valid_description,
    "a node that drops its description when a declaration is half typed",
    expected = "the node keeps its last valid description",
    check_declaration_edit(false),
);

// ── Eject ─────────────────────────────────────────────────────────────────────────────────────────────────────────

fn lightness(id: u32, l: f64) -> Tree {
    Tree {
        id,
        output: ValueType::F32,
        call: Call {
            name: "lightness".into(),
            function: "f32".into(),
            args: vec![Arg::Param(0)],
            params: vec![Param::free("L", UniformType::F32, vec![l])],
        },
        inputs: vec![],
    }
}

fn grey(id: u32, input: Tree) -> Tree {
    Tree {
        id,
        output: ValueType::Vec3,
        call: Call {
            name: "grey".into(),
            function: "ramp_grey".into(),
            args: vec![Arg::Input(0)],
            params: vec![],
        },
        inputs: vec![input],
    }
}

fn generated() -> render::codegen::Generated {
    generate(Slot::Colour, &grey(2, lightness(1, 0.5))).unwrap_or_else(|e| panic!("{e}"))
}

fn check_eject(break_it: bool) {
    let g = generated();
    let mut e = Ejected::new(g.clone());
    e.eject_all();
    if break_it {
        // A revert that leaves a trailing newline behind is not the generated source.
        e.edit(1, &format!("{}\n", g.functions[0].text))
            .unwrap_or_else(|e| panic!("{e}"));
        e.revert(2).unwrap_or_else(|e| panic!("{e}"));
    } else {
        e.revert_all();
    }
    assert!(
        e.source() == g.source,
        "eject then revert restores the generated source byte for byte"
    );

    let mut e = Ejected::new(g.clone());
    let text = e.eject(1).unwrap_or_else(|e| panic!("{e}")).to_owned();
    assert_eq!(text, g.functions[0].text, "ejected as generated");
    assert_eq!(e.source(), g.source, "ejecting alone changes nothing");
    let edited = text.replace(
        "return f32(uniforms.node_1_L);",
        "return f32(uniforms.node_1_L) * 0.5;",
    );
    assert_ne!(edited, text, "the edit changed the text");
    e.edit(1, &edited).unwrap_or_else(|e| panic!("{e}"));
    let spliced = e.source();
    assert!(spliced.contains(&edited));
    assert!(
        spliced.contains(&g.functions[1].text),
        "the peer's function is byte for byte"
    );
    assert!(
        spliced.contains("// @uniform node_1_L"),
        "its control stays live"
    );
    e.revert(1).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(e.source(), g.source);

    assert_eq!(e.dependencies(2), Ok(vec![1]), "node 2 calls node 1");
    assert!(!e.is_ejected(1));
    assert!(
        e.edit(1, &g.functions[0].text).is_err(),
        "an edit of a node that is not ejected is refused"
    );
    e.eject(1).unwrap_or_else(|e| panic!("{e}"));
    assert!(
        e.edit(1, "fn other() {}").is_err(),
        "text that loses the function's name is refused: its peers call it by that name"
    );

    // The tree loses node 1: its ejected text is reported, not silently dropped or kept.
    let smaller =
        generate(Slot::Colour, &grey(2, lightness(3, 0.5))).unwrap_or_else(|e| panic!("{e}"));
    let drift = e.regenerate(smaller);
    assert!(drift.contains(&Drift::Gone { id: 1 }), "{drift:?}");
}

#[test]
fn custom_node_fallback_eject_then_revert_is_byte_identical() {
    check_eject(false);
}

negative_control!(
    custom_node_fallback_eject_then_revert_is_byte_identical,
    "a revert that leaves the ejected text behind",
    expected = "byte for byte",
    check_eject(true),
);

/// Edit of a node that is not ejected is refused; the control ejects it first, so the edit is taken.
fn check_edit_refused(eject_first: bool) {
    let g = generated();
    let mut e = Ejected::new(g.clone());
    if eject_first {
        e.eject(1).unwrap_or_else(|e| panic!("{e}"));
    }
    assert!(
        e.edit(1, &g.functions[0].text).is_err(),
        "an edit of a node that is not ejected is refused"
    );
}

#[test]
fn custom_node_fallback_edit_needs_an_ejected_node() {
    check_edit_refused(false);
}

negative_control!(
    custom_node_fallback_edit_needs_an_ejected_node,
    "the node is ejected, so its edit is taken",
    expected = "is refused",
    check_edit_refused(true),
);

/// Drift. Pass criteria, written before the run: node 2's function (it calls `node_1`) is ejected and edited, then
/// the tree is regenerated unchanged. Editing its call to `node_9` reports exactly `Missing { id: 2, callee: 9 }`
/// beside the `Rewired` that the lost call to node 1 is; dropping the call to `node_1` for a constant reports
/// exactly `Rewired { id: 2, ejected: [], generated: [1] }`; an edit that keeps the calls reports nothing. The
/// controls make the edit that keeps the calls, so the drift asserted is not there.
fn drifted(edit: fn(&str) -> String) -> Vec<Drift> {
    let g = generated();
    let mut e = Ejected::new(g.clone());
    let text = e.eject(2).unwrap_or_else(|e| panic!("{e}")).to_owned();
    assert!(text.contains("node_1("), "node 2 calls node 1: {text}");
    let edited = edit(&text);
    e.edit(2, &edited).unwrap_or_else(|e| panic!("{e}"));
    e.regenerate(g)
}

fn calls_node_9(text: &str) -> String {
    text.replace("node_1(", "node_9(")
}

fn drops_the_call(text: &str) -> String {
    let at = text.find("node_1(").expect("a call");
    let close = at + text[at..].find(')').expect("its end") + 1;
    format!("{}vec3<f32>(0.5){}", &text[..at], &text[close..])
}

fn keeps_the_calls(text: &str) -> String {
    format!("{text}\n")
}

fn check_missing(edit: fn(&str) -> String) {
    let drift = drifted(edit);
    assert!(
        drift.contains(&Drift::Missing { id: 2, callee: 9 }),
        "a call to a function no node has is reported: {drift:?}"
    );
}

fn check_rewired(edit: fn(&str) -> String) {
    let drift = drifted(edit);
    assert_eq!(
        drift,
        vec![Drift::Rewired {
            id: 2,
            ejected: vec![],
            generated: vec![1]
        }],
        "an ejected function whose calls differ from the tree's is reported as rewired"
    );
}

#[test]
fn custom_node_fallback_drift_reports_a_missing_callee() {
    check_missing(calls_node_9);
    assert!(
        drifted(keeps_the_calls).is_empty(),
        "no edit of the calls, no drift"
    );
}

negative_control!(
    custom_node_fallback_drift_reports_a_missing_callee,
    "the edit keeps its calls",
    expected = "no node has is reported",
    check_missing(keeps_the_calls),
);

#[test]
fn custom_node_fallback_drift_reports_rewiring() {
    check_rewired(drops_the_call);
}

negative_control!(
    custom_node_fallback_drift_reports_rewiring,
    "the edit keeps its calls",
    expected = "reported as rewired",
    check_rewired(keeps_the_calls),
);

// ── Values follow a declaration ───────────────────────────────────────────────────────────────────────────────────

fn uniform(name: &str, ty: UniformType, default: &[f64], range: Option<(f64, f64)>) -> Uniform {
    Uniform {
        name: name.into(),
        ty,
        default: default.to_vec(),
        range,
        any_kind: false,
    }
}

type Reconciler = fn(
    &[Uniform],
    &BTreeMap<String, Vec<f64>>,
    &Remembered,
    &[Uniform],
) -> render::custom::Reconciled;

fn check_values(reconcile: Reconciler) {
    let old = [
        uniform("k", UniformType::F32, &[1.0], Some((0.0, 4.0))),
        uniform("n", UniformType::F32, &[1.0], None),
        uniform("gone", UniformType::F32, &[1.0], None),
    ];
    let values: BTreeMap<String, Vec<f64>> = [
        ("k".to_owned(), vec![3.0]),
        ("n".to_owned(), vec![5.0]),
        ("gone".to_owned(), vec![7.0]),
    ]
    .into();
    // `n` is first now, `k`'s range shrinks, `n` becomes an integer, `gone` is removed.
    let new = vec![
        uniform("n", UniformType::I32, &[1.0], None),
        uniform("k", UniformType::F32, &[1.0], Some((0.0, 2.0))),
    ];
    let r = reconcile(&old, &values, &Remembered::new(), &new);
    assert_eq!(
        r.values.get("k"),
        Some(&vec![2.0]),
        "clamped to the new range"
    );
    assert!(
        r.notes.contains(&Note::Clamped {
            name: "k".into(),
            was: vec![3.0],
            now: vec![2.0]
        }),
        "and said so: {:?}",
        r.notes
    );
    assert_eq!(r.values.get("n"), None, "a changed type takes the default");
    assert!(r.notes.contains(&Note::Reset {
        name: "n".into(),
        was: vec![5.0]
    }));
    assert!(
        r.remembered.contains_key("gone"),
        "a removed value is remembered"
    );
    // The uniform comes back.
    let back = [
        new[1].clone(),
        uniform("gone", UniformType::F32, &[1.0], None),
    ];
    let r2 = reconcile(&new, &r.values, &r.remembered, &back);
    assert_eq!(
        r2.values.get("gone"),
        Some(&vec![7.0]),
        "a removed uniform's value comes back with it"
    );
}

#[test]
fn custom_node_fallback_values_follow_the_declaration() {
    check_values(reconcile);
}

#[cfg_attr(not(feature = "controls"), allow(dead_code))]
/// A reconciler that carries the values by position, as a declaration read by index would.
fn by_position(
    old: &[Uniform],
    values: &BTreeMap<String, Vec<f64>>,
    remembered: &Remembered,
    new: &[Uniform],
) -> render::custom::Reconciled {
    let mut out = reconcile(old, &BTreeMap::new(), remembered, new);
    for (o, n) in old.iter().zip(new) {
        if let Some(v) = values.get(&o.name) {
            out.values.insert(n.name.clone(), v.clone());
        }
    }
    out
}

negative_control!(
    custom_node_fallback_values_follow_the_declaration,
    "values carried by position, not by name",
    expected = "clamped to the new range",
    check_values(by_position),
);

// ── New from template ─────────────────────────────────────────────────────────────────────────────────────────────

const FIELD: &str = "fn source(ctx: Ctx) -> Field { return Field(1.0, 0.0, 0.0, 0.0); }";
const SHOW: &str = "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(0.5); }";
const BRIGHT: &str = "fn brightness(ctx: Ctx) -> f32 { return 0.5; }";
const COMBINE: &str = "fn combine(rgb: vec3<f32>, b: f32) -> vec3<f32> { return rgb * b; }";
const POST: &str = "fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb; }";

/// Every kind of node wired into one stain, `kind`'s occupant being `text`.
fn stain_with(kind: Kind, text: &str) -> Stain {
    let mine = |k: Kind, fallback: &str| {
        Occupant::Custom(if k == kind { text } else { fallback }.to_owned())
    };
    Stain::new(vec![
        node(Kind::Source, mine(Kind::Source, FIELD), &[]),
        node(Kind::Colour, mine(Kind::Colour, SHOW), &[Some(0)]),
        node(Kind::Brightness, mine(Kind::Brightness, BRIGHT), &[Some(0)]),
        node(
            Kind::Combiner,
            mine(Kind::Combiner, COMBINE),
            &[Some(1), Some(2)],
        ),
        node(Kind::Post, mine(Kind::Post, POST), &[Some(3)]),
        node(Kind::Out, Occupant::None, &[Some(4)]),
    ])
    .unwrap_or_else(|e| panic!("{kind:?}: {e}"))
}

fn check_templates(make: fn(Kind, &str, &str) -> Option<String>) {
    for kind in [
        Kind::Source,
        Kind::Colour,
        Kind::Brightness,
        Kind::Combiner,
        Kind::Post,
    ] {
        let text = make(kind, "mine/new", "New").expect("a template");
        let d = assemble::declaration(kind, &Occupant::Custom(text.clone()))
            .unwrap_or_else(|e| panic!("{kind:?}: {e}"));
        assert_eq!(d.described.id.as_deref(), Some("mine/new"), "{kind:?}");
        assert_eq!(d.uniforms.len(), 1, "{kind:?}");
        // The slot's function is there, in the node's own slot, so a stain takes the node.
        let defined = assemble::defined_functions(&text).unwrap_or_else(|e| panic!("{e}"));
        assert!(
            defined.iter().any(|f| Some(f.as_str()) == kind.slot()),
            "{kind:?}: a template defines its slot's function"
        );
        // And the body compiles: the stain holding it assembles.
        if let Err(e) = assemble::assemble(&stain_with(kind, &text), Tier::FULL) {
            panic!("{kind:?}: a template assembles into a stain: {e}");
        }
    }
    assert!(make(Kind::Out, "mine/new", "New").is_none());
}

#[test]
fn custom_node_fallback_every_template_is_a_node() {
    check_templates(template);
}

negative_control!(
    custom_node_fallback_every_template_is_a_node,
    "a template that is the colour slot's for every kind",
    expected = "defines its slot's function",
    check_templates(|kind, id, name| template(Kind::Colour, id, name).filter(|_| kind != Kind::Out)),
);

#[test]
fn custom_node_fallback_every_template_compiles() {
    check_templates(template);
}

negative_control!(
    custom_node_fallback_every_template_compiles,
    "a template whose body reads a uniform it does not declare",
    expected = "a template assembles into a stain",
    check_templates(|kind, id, name| template(kind, id, name)
        .map(|t| t.replace("uniforms.scale", "uniforms.missing"))),
);
