//! A node's fragment key hashes the code it runs, whoever wrote it. Every shipped source, shown in its slot, has the key
//! of the same graph with its file's text pasted in as a custom; every field's source has the key of its generated
//! text pasted in. So a shipped source and its pasted copy compile once (the cache compiles once per key:
//! `pipeline_runtime`'s `fragment_cache_shipped_and_pasted_compile_once`). The control writes the shipped source's
//! name into the key, as a key by id would.

use render::assemble::{self, Kind, Node, Occupant, Stain};
use render::registry::{self, ZERO};
use validation::negative_control;

fn node(kind: Kind, occupant: Occupant, inputs: &[Option<usize>]) -> Node {
    Node {
        kind,
        occupant,
        inputs: inputs.to_vec(),
    }
}

/// A node of `kind` and `occupant`, every field input fed by node 0, the zero source; a post's colour by `colour`.
fn fed(kind: Kind, occupant: Occupant, colour: Option<usize>) -> Node {
    let d = assemble::declaration(kind, &occupant).unwrap_or_else(|e| panic!("{e}"));
    let mut inputs: Vec<Option<usize>> = vec![Some(0); d.inputs.len()];
    if kind == Kind::Post {
        inputs.insert(0, colour);
    }
    node(kind, occupant, &inputs)
}

const SHOW: &str = "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.inputs[0].x); }";

fn pass_through() -> Occupant {
    Occupant::BuiltIn("pass_through".into())
}

/// The graph that shows `occupant` in a node of `kind`, with the zero source, the pass-through combiner and `OUT`.
fn showing(kind: Kind, occupant: Occupant) -> Vec<Node> {
    let zero = node(Kind::Source, Occupant::BuiltIn(ZERO.to_owned()), &[]);
    let show = || fed(Kind::Colour, Occupant::Custom(SHOW.into()), None);
    let out = |from| node(Kind::Out, Occupant::None, &[Some(from)]);
    match kind {
        Kind::Source => vec![
            node(Kind::Source, occupant, &[]),
            show(),
            node(Kind::Combiner, pass_through(), &[Some(1), None]),
            out(2),
        ],
        Kind::Colour => vec![
            zero,
            fed(Kind::Colour, occupant, None),
            node(Kind::Combiner, pass_through(), &[Some(1), None]),
            out(2),
        ],
        Kind::Brightness => vec![
            zero,
            fed(Kind::Brightness, occupant, None),
            node(Kind::Combiner, pass_through(), &[None, Some(1)]),
            out(2),
        ],
        Kind::Combiner => vec![
            zero,
            show(),
            node(Kind::Combiner, occupant, &[Some(1), None]),
            out(2),
        ],
        Kind::Post => vec![
            zero,
            show(),
            node(Kind::Combiner, pass_through(), &[Some(1), None]),
            fed(Kind::Post, occupant, Some(2)),
            out(3),
        ],
        Kind::Out => unreachable!("OUT has no occupant"),
    }
}

fn key(g: Vec<Node>) -> u64 {
    Stain::new(g)
        .unwrap_or_else(|e| panic!("{e}"))
        .canonical()
        .fragment_key()
}

/// Each shipped source and each field's source, by its name, in a node of its slot, with the text it runs.
fn shipped() -> Vec<(String, Kind, Occupant, String)> {
    let mut out = Vec::new();
    for e in registry::registry().unwrap_or_else(|e| panic!("{e}")) {
        let occupant = Occupant::BuiltIn(e.name().to_owned());
        out.push((e.id.clone(), e.slot, occupant, e.source.clone()));
    }
    for f in assemble::source_fields().unwrap_or_else(|e| panic!("{e}")) {
        let occupant = Occupant::Field(f.clone());
        let text = assemble::occupant_source(Kind::Source, &occupant)
            .unwrap_or_else(|e| panic!("{e}"))
            .expect("a field's source");
        out.push((format!("ctx.{f}"), Kind::Source, occupant, text));
    }
    out
}

/// Every shipped source and its pasted copy are one key.
fn check_one_key() {
    let all = shipped();
    assert!(all.len() > 87, "{} shipped sources and fields", all.len());
    for (name, kind, occupant, text) in &all {
        let shipped = key(showing(*kind, occupant.clone()));
        let pasted = key(showing(*kind, Occupant::Custom(text.clone())));
        assert!(
            shipped == pasted,
            "{name}: the shipped source and its pasted copy have two keys ({shipped:016x}, {pasted:016x})"
        );
    }
}

#[test]
fn a_shipped_source_and_its_pasted_copy_are_one_key() {
    check_one_key();
}

negative_control!(
    a_shipped_source_and_its_pasted_copy_are_one_key,
    "the shipped source's id written into the key",
    expected = "have two keys",
    assemble::probe::id_in_key_in(check_one_key)
);

/// One code is one key: the pasted copy of `pass_through` given as the combiner of a graph keys as the shipped one does,
/// and other code does not.
fn check_other_code_other_key(edit: &dyn Fn(&str) -> String) {
    let shipped = registry::named(Kind::Combiner, "pass_through").unwrap_or_else(|e| panic!("{e}"));
    let edited = edit(&shipped.source);
    let a = key(showing(Kind::Combiner, pass_through()));
    let b = key(showing(Kind::Combiner, Occupant::Custom(edited)));
    assert!(a != b, "other code keys as the shipped source");
}

#[test]
fn other_code_is_another_key() {
    check_other_code_other_key(&|s| s.replace("return rgb", "return 0.5 * rgb"));
}

negative_control!(
    other_code_is_another_key,
    "the same code with only a label edited",
    expected = "other code keys as the shipped source",
    check_other_code_other_key(&|s| s.replace("@name ", "@name Other "))
);
