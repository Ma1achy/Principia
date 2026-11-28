//! A field ramp in a stain is the shipped `field_ramp` node with its parameters set. Pass criteria, written before the
//! runs:
//!
//! - For every scalar field and each slot (the colour ramp; the brightness ramp with its post), the stain whose ramp
//!   is the shipped node with the field's default parameters assembles, in the presentation and the debug view, to the
//!   code, and reads the fields, of the stain holding the ramp's own generated text as it was
//!   (`render/tests/data/field_ramps_before.txt`). Code equal is picture equal: the ramp is the
//!   only node that differs. Control: the node made as the next field assembles to other code.
//! - A stain saved before the change, holding a ramp as custom text (the only form a ramp had), opens, holds that text
//!   unchanged, and draws what the node of the same field draws. Control: another field's node does not.
//! - A ramp node saves the field it reads by its name and reads it back by name, for a field the source list does not
//!   hold too (`m0`, read through the initial conditions). Control: a stain with the name taken out holds no field.
//! - The three shipped ramp files are in the library, each with the controls its source declares.

use std::collections::BTreeMap;

use engine::contract::stain::assembled::{self, library};
use engine::contract::stain::describe;
use engine::contract::stain::graph::{NodeId, NodeKind, Occupant, StainGraph};
use render::assemble::{self, View};
use render::codegen::Slot;
use render::colour::defaults::{self, Resolve};
use render::colour::field_ramp::FieldRamp;
use validation::negative_control;

const BEFORE: &str = include_str!("../../render/tests/data/field_ramps_before.txt");

fn sim_key(name: &str) -> Option<f64> {
    matches!(name, "eps_E" | "eps_L").then_some(1.0 / 1_048_576.0)
}

/// The generated text of each `<field> <slot>`, as it was, comments dropped.
fn before() -> BTreeMap<String, String> {
    let mut out: BTreeMap<String, String> = BTreeMap::new();
    let mut name = String::new();
    for line in BEFORE.lines() {
        match line.strip_prefix("=== ") {
            Some(n) => name = n.to_owned(),
            None => {
                let t = out.entry(name.clone()).or_default();
                t.push_str(line);
                t.push('\n');
            }
        }
    }
    out
}

fn code(source: &str) -> String {
    source
        .lines()
        .map(|l| l.split_once("//").map_or(l, |(c, _)| c).trim_end())
        .filter(|l| !l.trim().is_empty())
        .map(|l| format!("{l}\n"))
        .collect()
}

/// The default ramp of `ramp`'s field as the shipped node of `kind`, with its parameters set as the window sets them.
fn node(g: &mut StainGraph, kind: NodeKind, ramp: &FieldRamp) -> NodeId {
    let id = g
        .add(kind, Occupant::Builtin("field_ramp".into()))
        .unwrap_or_else(|e| panic!("{e}"));
    let params = assembled::field_ramp_params(kind, ramp.field.field, &sim_key)
        .unwrap_or_else(|e| panic!("{e}"));
    for (name, value) in params {
        g.set_param(id, &name, value)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
    }
    id
}

/// A stain of the field `field` read by a source, into `ramp` in its slot, the combiner and `OUT`; a brightness's post
/// is its style post. `ramp` is the occupant of the slot, as the node with parameters or as text.
fn stain(
    field: &str,
    slot: Slot,
    ramp: &dyn Fn(&mut StainGraph, NodeKind) -> NodeId,
    post: &dyn Fn(&mut StainGraph) -> NodeId,
) -> StainGraph {
    let mut g = StainGraph::new();
    let (combiner, out) = (NodeId(0), NodeId(1));
    g.set_occupant(combiner, Occupant::Builtin("pass_through".into()))
        .unwrap_or_else(|e| panic!("{e}"));
    let s = g
        .add(NodeKind::Source, Occupant::Field(field.to_owned()))
        .unwrap_or_else(|e| panic!("{e}"));
    let kind = match slot {
        Slot::Colour => NodeKind::Colour,
        Slot::Brightness => NodeKind::Brightness,
    };
    let r = ramp(&mut g, kind);
    g.connect(s, r, 0).unwrap_or_else(|e| panic!("{e}"));
    g.connect(r, combiner, usize::from(slot == Slot::Brightness))
        .unwrap_or_else(|e| panic!("{e}"));
    if slot == Slot::Brightness {
        let p = post(&mut g);
        g.connect(combiner, p, 0).unwrap_or_else(|e| panic!("{e}"));
        g.connect(p, out, 0).unwrap_or_else(|e| panic!("{e}"));
    } else {
        g.connect(combiner, out, 0)
            .unwrap_or_else(|e| panic!("{e}"));
    }
    g
}

/// What a stain assembles to in both views: the code, comments dropped, and the fields read.
fn assembled(g: &StainGraph) -> Vec<(String, Vec<String>)> {
    let stain = g.lower().unwrap_or_else(|e| panic!("{e}"));
    [View::Presentation, View::Debug]
        .into_iter()
        .map(|v| {
            let f = assemble::assemble_kept(&stain, assemble::Tier::FULL, v)
                .unwrap_or_else(|e| panic!("{e}"));
            (code(&f.source), f.fields.clone())
        })
        .collect()
}

/// Whether, for every field and slot, the stain `made` gives holds what the stain of the generated text does.
fn check(made: &dyn Fn(&str, Slot, &FieldRamp) -> StainGraph) {
    let (old, l) = (before(), ledger::payload::ledger());
    let resolve = Resolve { sim_key: &sim_key };
    let mut rows = 0;
    for e in ledger::gen::validate(&l).unwrap_or_else(|e| panic!("{e}")) {
        for slot in [Slot::Colour, Slot::Brightness] {
            let Ok(ramp) = defaults::field_ramp(&e, slot, &resolve) else {
                continue;
            };
            let slot_name = slot.kind().name();
            let text = |what: &str| Occupant::Custom(old[&format!("{} {what}", e.name)].clone());
            // The ramp reads its own field; the source beside it is the same in both stains.
            let field = "state";
            let was = stain(
                field,
                slot,
                &|g, k| g.add(k, text(slot_name)).unwrap_or_else(|e| panic!("{e}")),
                &|g| {
                    g.add(NodeKind::Post, text("post"))
                        .unwrap_or_else(|e| panic!("{e}"))
                },
            );
            let now = made(field, slot, &ramp);
            assert_eq!(
                assembled(&now),
                assembled(&was),
                "{} {slot_name}: the stain with the node does not assemble as the generated ramp did",
                e.name
            );
            rows += 1;
        }
    }
    println!("stains checked against the generated ramps: {rows}");
}

fn with_node(field: &str, slot: Slot, ramp: &FieldRamp) -> StainGraph {
    stain(field, slot, &|g, k| node(g, k, ramp), &|g| {
        node(g, NodeKind::Post, ramp)
    })
}

#[test]
fn every_stain_with_a_ramp_assembles_as_before() {
    check(&with_node);
}

negative_control!(
    every_stain_with_a_ramp_assembles_as_before,
    "the node made as the next field",
    expected = "does not assemble as the generated ramp did",
    {
        let l = ledger::payload::ledger();
        let resolve = Resolve { sim_key: &sim_key };
        let all: Vec<FieldRamp> = ledger::gen::validate(&l)
            .unwrap_or_else(|e| panic!("{e}"))
            .iter()
            .filter_map(|e| defaults::field_ramp(e, Slot::Colour, &resolve).ok())
            .collect();
        check(&|field, slot, ramp| {
            let at = all
                .iter()
                .position(|r| r.field.field == ramp.field.field)
                .unwrap_or(0);
            let other = &all[(at + 1) % all.len()];
            let mut shifted = ramp.clone();
            shifted.field = other.field.clone();
            with_node(field, slot, &shifted)
        })
    }
);

// ── Saved stains ──────────────────────────────────────────────────────────────────────────────────────────────

/// An old stain: the field's source and `ramp` as custom text, with its code by value.
fn saved_before(field: &str, ramp_text: &str) -> String {
    let mut g = StainGraph::new();
    g.set_occupant(NodeId(0), Occupant::Builtin("pass_through".into()))
        .unwrap_or_else(|e| panic!("{e}"));
    let s = g
        .add(NodeKind::Source, Occupant::Field(field.to_owned()))
        .unwrap_or_else(|e| panic!("{e}"));
    let r = g
        .add(NodeKind::Colour, Occupant::Custom(ramp_text.to_owned()))
        .unwrap_or_else(|e| panic!("{e}"));
    for (from, to) in [(s, r), (r, NodeId(0)), (NodeId(0), NodeId(1))] {
        g.connect(from, to, 0).unwrap_or_else(|e| panic!("{e}"));
    }
    serde_json::to_string_pretty(&g).unwrap_or_else(|e| panic!("{e}"))
}

/// Whether an old stain of `field`'s ramp, written as custom text, opens holding its text and draws as `against`'s node.
fn check_old_opens(field: &str, against: &str) {
    let l = ledger::payload::ledger();
    let resolve = Resolve { sim_key: &sim_key };
    let entry = |name: &str| {
        ledger::gen::validate(&l)
            .unwrap_or_else(|e| panic!("{e}"))
            .into_iter()
            .find(|e| e.name == name)
            .unwrap_or_else(|| panic!("no field {name}"))
    };
    let ramp = defaults::field_ramp(&entry(field), Slot::Colour, &resolve)
        .unwrap_or_else(|e| panic!("{e}"));
    // The text as the generator wrote it, comments and all, is in the old stain; its code is the fixture's.
    let old_text = before()[&format!("{field} colour")].clone();
    let json = saved_before(field, &old_text);
    let opened: StainGraph = serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
    let held = opened
        .nodes()
        .find(|n| n.kind == NodeKind::Colour)
        .unwrap_or_else(|| panic!("no colour node"));
    assert_eq!(
        held.occupant,
        Occupant::Custom(old_text.clone()),
        "the old stain holds its ramp's text unchanged"
    );
    let other = defaults::field_ramp(&entry(against), Slot::Colour, &resolve)
        .unwrap_or_else(|e| panic!("{e}"));
    let drawn = |g: &StainGraph| assembled(g);
    let fresh = stain(
        field,
        Slot::Colour,
        &|g, k| {
            node(
                g,
                k,
                &if against == field {
                    ramp.clone()
                } else {
                    other.clone()
                },
            )
        },
        &|g| node(g, NodeKind::Post, &ramp),
    );
    assert_eq!(
        drawn(&opened),
        drawn(&fresh),
        "{field}: the old stain does not draw what the node of {against} draws"
    );
}

#[test]
fn a_stain_saved_with_an_old_ramp_opens_and_draws_as_before() {
    for f in ["ftle", "d_min", "energy_drift", "t_end_step", "theta"] {
        check_old_opens(f, f);
    }
}

negative_control!(
    a_stain_saved_with_an_old_ramp_opens_and_draws_as_before,
    "an old ramp compared with another field's node",
    expected = "does not draw what the node of",
    check_old_opens("ftle", "d_min")
);

// ── A field of the ramp's list, saved by name ─────────────────────────────────────────────────────────────────

fn ramp_of(field: &str) -> FieldRamp {
    let l = ledger::payload::ledger();
    let e = ledger::gen::validate(&l)
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .find(|e| e.name == field)
        .unwrap_or_else(|| panic!("no field {field}"));
    defaults::field_ramp(&e, Slot::Colour, &Resolve { sim_key: &sim_key })
        .unwrap_or_else(|e| panic!("{e}"))
}

/// `field`'s ramp node saved: its text holds the field's name, and opens to the same graph, drawing the same.
fn check_by_name(edit: &dyn Fn(String) -> String) {
    for field in ["m0", "length", "ftle"] {
        let g = with_node("state", Slot::Colour, &ramp_of(field));
        let json = edit(serde_json::to_string_pretty(&g).unwrap_or_else(|e| panic!("{e}")));
        assert!(
            json.contains(&format!("\"field\": \"{field}\"")),
            "the stain does not hold `{field}` by name"
        );
        let back: StainGraph = serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(back, g, "{field}: a stain saved and opened is another");
        assert_eq!(
            assembled(&back),
            assembled(&g),
            "{field}: it draws other code"
        );
    }
}

#[test]
fn a_ramp_saves_its_field_by_name() {
    check_by_name(&|j| j);
}

negative_control!(
    a_ramp_saves_its_field_by_name,
    "the saved name taken out",
    expected = "does not hold",
    check_by_name(&|j| j.replace("\"m0\"", "\"nothing\""))
);

// ── In the library ────────────────────────────────────────────────────────────────────────────────────────────

fn check_library(drop: Option<&str>) {
    let entries = library().unwrap_or_else(|e| panic!("{e}"));
    for (kind, name) in [
        (NodeKind::Colour, "field_ramp"),
        (NodeKind::Brightness, "field_ramp"),
        (NodeKind::Post, "field_ramp"),
    ] {
        let found = entries
            .iter()
            .find(|e| e.slot == kind && e.name == name && drop != Some(&e.id));
        let entry = found.unwrap_or_else(|| panic!("the library has no {kind:?} `{name}`"));
        let d = describe::describe(kind, &Occupant::Builtin(name.into()), &BTreeMap::new())
            .unwrap_or_else(|e| panic!("{e}"));
        let titles: Vec<&str> = d.controls.iter().map(|c| c.label.as_str()).collect();
        assert!(
            titles.contains(&"Field") && titles.contains(&"Curve"),
            "{}: the field and the curve are controls: {titles:?}",
            entry.id
        );
    }
}

#[test]
fn the_three_ramps_are_in_the_library() {
    check_library(None);
}

negative_control!(
    the_three_ramps_are_in_the_library,
    "the colour ramp taken out of the library",
    expected = "the library has no",
    check_library(Some("colour/field_ramp"))
);

// ── What a ramp needs follows its field ───────────────────────────────────────────────────────────────────────

/// The ramp node of `field`, alone in a graph, and the occupant its needs are read from: its made text, or, as a
/// control, the shipped file it is made from.
fn needs_of(
    field: &str,
    made: bool,
) -> (
    render::assemble::matters::Needs,
    Option<describe::MeasuredRange>,
) {
    let mut g = StainGraph::new();
    let id = node(&mut g, NodeKind::Colour, &ramp_of(field));
    let occupant = if made {
        g.made(id).unwrap_or_else(|e| panic!("{e}"))
    } else {
        Occupant::Builtin("field_ramp".into())
    };
    (
        describe::needs(NodeKind::Colour, &occupant).unwrap_or_else(|e| panic!("{e}")),
        describe::measured_range(NodeKind::Colour, &occupant).unwrap_or_else(|e| panic!("{e}")),
    )
}

fn check_needs(made: bool) {
    // The FTLE's ramp needs the FTLE and its measured range; `d_min`'s, on `log`, needs neither; the signed `S` on a
    // symmetric measured range reads its own.
    let (ftle, range) = needs_of("ftle", made);
    assert!(ftle.ftle, "the FTLE's ramp does not need the FTLE");
    assert_eq!(range.and_then(|r| r.field).as_deref(), Some("ftle"));
    let (d_min, range) = needs_of("d_min", made);
    assert!(!d_min.ftle, "a d_min ramp is told it needs the FTLE");
    assert_eq!(
        range, None,
        "a d_min ramp is told it takes a measured range"
    );
    let (_, range) = needs_of("S", made);
    assert_eq!(range.and_then(|r| r.field).as_deref(), Some("S"));
}

#[test]
fn a_ramps_needs_follow_its_field() {
    check_needs(true);
}

negative_control!(
    a_ramps_needs_follow_its_field,
    "the needs read from the shipped file, not the node as it is made",
    expected = "a d_min ramp is told",
    check_needs(false)
);

/// The block of a ramp node holds the measured range only where its curve reads one.
fn check_block(made: bool) {
    for (field, range) in [("ftle", true), ("d_min", false)] {
        let mut g = StainGraph::new();
        let id = node(&mut g, NodeKind::Colour, &ramp_of(field));
        let names: Vec<String> = if made {
            g.block(id)
                .unwrap_or_else(|e| panic!("{e}"))
                .into_iter()
                .map(|u| u.name)
                .collect()
        } else {
            g.uniforms(id)
                .unwrap_or_else(|e| panic!("{e}"))
                .into_iter()
                .map(|u| u.name)
                .collect()
        };
        assert_eq!(
            names.iter().any(|n| n == "u_range"),
            range,
            "{field}: the block's members are {names:?}, a range only where the curve reads one"
        );
    }
}

#[test]
fn a_ramps_block_holds_what_its_curve_reads() {
    check_block(true);
}

negative_control!(
    a_ramps_block_holds_what_its_curve_reads,
    "the shipped file's uniforms, not the made block's",
    expected = "a range only where the curve reads one",
    check_block(false)
);
