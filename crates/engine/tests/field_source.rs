//! A field source is the one shipped source `field` with a `field` parameter. Pass criteria, written before the runs:
//!
//! - For every readable field, a graph whose source is the shipped `field` with its parameter set to that field (and
//!   one that holds `Occupant::Field(f)`) assembles to the code the field's own generated source assembled to (the
//!   oracle below is that generator as it was), comments dropped, equal as text; its out-port's subtype is the
//!   field's. Control: the parameter left at its default, so the subtype and code are the default field's.
//! - A stain saved with a per-field source, as `{"field": f}` and as the old generated text under `source/ctx.<f>`,
//!   opens as that field and assembles to the same code; a text under that id whose code is another field's stays as
//!   saved, and is not read as the field. Control: the other field's text taken for the field's.
//! - A field source is saved as the shipped `field` source with its field by name, and opens to the same code.

use engine::contract::stain::assembled::{assemble, source_fields, Tier};
use engine::contract::stain::graph::{NodeId, NodeKind, Occupant, StainGraph};
use engine::contract::stain::saved::hash;
use render::assemble as asm;
use validation::negative_control;

/// The text a field's source was generated as before it was one shipped file, declarations included: the oracle.
fn oracle(f: &str) -> String {
    let l = ledger::payload::ledger();
    let entries = ledger::gen::validate(&l).unwrap_or_else(|e| panic!("{e}"));
    let ty = ledger::gen::read::members(&l.words, &entries)
        .into_iter()
        .find(|m| m.name == f)
        .map(|m| m.wgsl)
        .unwrap_or_else(|| panic!("no field {f}"));
    let declared = |about: &str| {
        format!("// @name ctx.{f}\n// @group Source\n// @about {about}\n// @maps ctx.{f}\n// @glyph dropdown\n")
    };
    if ty == "vec3<f32>" {
        return format!(
            "// The built-in source of `{f}`: the vector field in `.xyz` (render_gui_spec Part II §3.1, §5).\n{}fn source(ctx: Ctx) -> Field {{ return Field(ctx.sample.{f}, 0.0); }}\n",
            declared(&format!("The read-side vector field {f}, in the field's xyz."))
        );
    }
    let value = match ty {
        "bool" => format!("select(0.0, 1.0, ctx.sample.{f})"),
        _ => format!("f32(ctx.sample.{f})"),
    };
    format!(
        "// The built-in source of `{f}`: the field in `.x` (render_gui_spec Part II §3.1).\n{}fn source(ctx: Ctx) -> Field {{ return Field({value}, 0.0, 0.0, 0.0); }}\n",
        declared(&format!("The read-side field {f}, in the field's x."))
    )
}

const COLOUR: &str = "fn colour(ctx: Ctx) -> vec3<f32> { return ctx.inputs[0].xyz; }";

/// A source `occupant` into a colour, the pass-through combiner and `OUT`; the source's id.
fn graph(occupant: Occupant) -> (StainGraph, NodeId) {
    let mut g = StainGraph::new();
    let (combiner, out) = (NodeId(0), NodeId(1));
    g.set_occupant(combiner, Occupant::Builtin("pass_through".into()))
        .expect("combiner");
    let s = g.add(NodeKind::Source, occupant).expect("source");
    let c = g
        .add(NodeKind::Colour, Occupant::Custom(COLOUR.into()))
        .expect("colour");
    for (from, to) in [(s, c), (c, combiner), (combiner, out)] {
        g.connect(from, to, 0).expect("a wire");
    }
    (g, s)
}

/// `source` with every `//` comment, trailing white space and blank line dropped.
fn code(source: &str) -> String {
    source
        .lines()
        .map(|l| l.split_once("//").map_or(l, |(c, _)| c).trim_end())
        .filter(|l| !l.trim().is_empty())
        .map(|l| format!("{l}\n"))
        .collect()
}

fn drawn(g: &StainGraph) -> String {
    code(
        &assemble(g, Tier::FULL)
            .unwrap_or_else(|e| panic!("{e}"))
            .code,
    )
}

/// The shipped `field` source with its parameter set to field number `at`.
fn shipped(at: usize) -> (StainGraph, NodeId) {
    let (mut g, s) = graph(Occupant::Builtin("field".into()));
    g.set_param(s, "field", vec![at as f64]).expect("a field");
    (g, s)
}

fn check_each_field(build: &dyn Fn(&str, usize) -> (StainGraph, NodeId)) {
    let names = source_fields().unwrap_or_else(|e| panic!("{e}"));
    assert!(names.len() > 20, "the readable fields: {}", names.len());
    for (k, f) in names.iter().enumerate() {
        let (old, _) = graph(Occupant::Custom(oracle(f)));
        let (g, s) = build(f, k);
        assert_eq!(drawn(&g), drawn(&old), "`{f}` draws other code than it did");
        let subtype = g.ports_of(s).expect("ports").outputs[0].subtype;
        let want = asm::field_subtype(f).ok().map(Into::into);
        assert_eq!(subtype, want, "`{f}`'s out-port subtype follows its field");
    }
}

#[test]
fn the_parameter_is_the_field_drawn() {
    check_each_field(&|_, k| shipped(k));
}

#[test]
fn a_field_occupant_is_the_shipped_source_with_that_field() {
    check_each_field(&|f, _| graph(Occupant::Field(f.to_owned())));
}

negative_control!(
    a_field_occupant_is_the_shipped_source_with_that_field,
    "the occupant holds the next field",
    expected = "draws other code than it did",
    check_each_field(&|f, _| {
        let names = source_fields().unwrap_or_else(|e| panic!("{e}"));
        let at = names.iter().position(|n| n == f).unwrap_or(0);
        graph(Occupant::Field(names[(at + 1) % names.len()].clone()))
    })
);

negative_control!(
    the_parameter_is_the_field_drawn,
    "the parameter left at its default, not set to the field",
    expected = "draws other code than it did",
    check_each_field(&|_, _| graph(Occupant::Builtin("field".into())))
);

fn check_subtypes(shifted: bool) {
    {
        let names = source_fields().unwrap_or_else(|e| panic!("{e}"));
        let kinds: Vec<_> = names.iter().map(|f| asm::field_subtype(f).ok()).collect();
        for (k, f) in names.iter().enumerate() {
            let (g, s) = shipped(k);
            let subtype = g.ports_of(s).expect("ports").outputs[0].subtype;
            let own = kinds[if shifted { (k + 1) % kinds.len() } else { k }].map(Into::into);
            assert_eq!(subtype, own, "`{f}`'s out-port subtype follows its field");
        }
    }
}

#[test]
fn the_subtype_follows_the_parameter() {
    check_subtypes(false);
}

negative_control!(
    the_subtype_follows_the_parameter,
    "the subtype read from the next field, not the parameter's",
    expected = "out-port subtype follows its field",
    check_subtypes(true)
);

// ── Old saved stains ──────────────────────────────────────────────────────────────────────────────────────────────

/// A stain saved with its source node's occupant as `occupant`, JSON; opened.
fn open_with(f: &str, occupant: serde_json::Value) -> (StainGraph, NodeId) {
    let (g, s) = graph(Occupant::Custom(oracle(f)));
    let mut v = serde_json::to_value(&g).unwrap_or_else(|e| panic!("{e}"));
    for n in v["nodes"].as_array_mut().expect("nodes") {
        if n["id"].as_u64() == Some(u64::from(s.0)) {
            n["occupant"] = occupant.clone();
        }
    }
    (
        serde_json::from_value(v).unwrap_or_else(|e| panic!("`{f}` does not open: {e}")),
        s,
    )
}

fn shader(id: &str, source: &str) -> serde_json::Value {
    serde_json::json!({"shader": {"id": id, "version": null, "hash": hash(source), "source": source}})
}

fn check_old(form: &dyn Fn(&str, &[String], usize) -> serde_json::Value) {
    let names = source_fields().unwrap_or_else(|e| panic!("{e}"));
    for (k, f) in names.iter().enumerate() {
        let (g, s) = open_with(f, form(f, &names, k));
        let (old, _) = graph(Occupant::Custom(oracle(f)));
        assert!(
            matches!(&g.node(s).expect("source").occupant, Occupant::Field(n) if n == f),
            "`{f}` opens as {:?}, not as the field",
            g.node(s).expect("source").occupant
        );
        assert_eq!(drawn(&g), drawn(&old), "`{f}` opens drawing other code");
    }
}

#[test]
fn old_per_field_sources_open_as_their_field() {
    check_old(&|f, _, _| serde_json::json!({"field": f}));
    check_old(&|f, _, _| shader(&format!("source/ctx.{f}"), &oracle(f)));
}

negative_control!(
    old_per_field_sources_open_as_their_field,
    "a text under the id whose code is another field's",
    expected = "not as the field",
    check_old(&|f, names, k| {
        let other = &names[(k + 1) % names.len()];
        shader(&format!("source/ctx.{f}"), &oracle(other))
    })
);

fn check_edited(edit: bool) {
    let f = "ftle";
    let mut text = oracle(f);
    if edit {
        text = text.replace("0.0, 0.0, 0.0)", "1.0, 0.0, 0.0)");
    }
    let (g, s) = open_with(f, shader(&format!("source/ctx.{f}"), &text));
    assert!(
        matches!(g.node(s).expect("source").occupant, Occupant::Pinned(_)),
        "an edited text stays as saved"
    );
    assert!(drawn(&g).contains("1.0, 0.0, 0.0)"), "and draws as saved");
}

#[test]
fn an_edited_old_source_stays_as_saved() {
    check_edited(true);
}

negative_control!(
    an_edited_old_source_stays_as_saved,
    "the text is not edited",
    expected = "an edited text stays as saved",
    check_edited(false)
);

// ── The new form ──────────────────────────────────────────────────────────────────────────────────────────────────

fn check_saved(shift: usize) {
    let names = source_fields().unwrap_or_else(|e| panic!("{e}"));
    for (k, f) in names.iter().enumerate() {
        let (g, s) = graph(Occupant::Field(f.clone()));
        let v = serde_json::to_value(&g).unwrap_or_else(|e| panic!("{e}"));
        let n = v["nodes"]
            .as_array()
            .expect("nodes")
            .iter()
            .find(|n| n["id"].as_u64() == Some(u64::from(s.0)))
            .expect("the source")
            .clone();
        assert_eq!(n["occupant"]["shader"]["id"], "source/field", "`{f}`");
        let saved = &names[(k + shift) % names.len()];
        assert_eq!(n["fields"]["field"], saved.as_str(), "`{f}`'s saved field");
        let back: StainGraph = serde_json::from_value(v).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(drawn(&back), drawn(&g), "`{f}` saved and opened");
        assert_eq!(
            back.ports_of(s).expect("ports"),
            g.ports_of(s).expect("ports")
        );
    }
}

#[test]
fn a_field_source_is_saved_as_the_shipped_source_with_its_field() {
    check_saved(0);
}

negative_control!(
    a_field_source_is_saved_as_the_shipped_source_with_its_field,
    "the saved field is the next one",
    expected = "saved field",
    check_saved(1)
);

// ── The title follows the parameter ───────────────────────────────────────────────────────────────────────────────

fn check_titles(shifted: bool) {
    use engine::contract::stain::describe::describe;
    let names = source_fields().unwrap_or_else(|e| panic!("{e}"));
    for (k, f) in names.iter().enumerate() {
        let mut params = std::collections::BTreeMap::new();
        params.insert("field".to_owned(), vec![k as f64]);
        let d = describe(
            NodeKind::Source,
            &Occupant::Builtin("field".into()),
            &params,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let want = &names[(k + usize::from(shifted)) % names.len()];
        assert_eq!(
            d.name,
            format!("ctx.{want}"),
            "`{f}`'s title follows the parameter"
        );
        let own = describe(
            NodeKind::Source,
            &Occupant::Field(f.clone()),
            &Default::default(),
        )
        .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(own.name, d.name, "`{f}`'s title as its own occupant");
    }
}

#[test]
fn the_title_follows_the_parameter() {
    check_titles(false);
}

negative_control!(
    the_title_follows_the_parameter,
    "the title taken from the next field",
    expected = "follows the parameter",
    check_titles(true)
);
