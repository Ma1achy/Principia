//! A field ramp is one shipped node per slot (`colour/field_ramp`, `brightness/field_ramp`, `post/field_ramp`) with the
//! field, curve, range, polarity and ramp as its parameters. Pass criteria, written before the runs:
//!
//! - For every readable scalar field and each slot (colour, brightness, and the brightness ramp's post), the shipped
//!   node made as the field's default ramp is, as code, comments, blank lines and trailing white space aside, the text
//!   the field's own generated ramp was (`tests/data/field_ramps_before.txt`, written by the generator as it was):
//!   equal as text, so the kept code of any stain holding it, in either view, is the code it was. Code equal is
//!   picture equal: nothing else about the occupant differs.
//! - Its declared uniforms (name, type, default, range) and its inputs are the generated ramp's
//!   (`tests/data/field_ramp_uniforms.txt`), the block bound to the shader and the values it starts at. The baked
//!   parameters are in no block.
//! - Every generated-ramp code digest in `tests/data/declared_code.txt` stays as it was: that file is not touched by
//!   this change, and `declared_code.rs` builds those rows through the node.
//! - Controls: the node taken as another field, another curve, the other polarity or another ramp is another code, and
//!   the comparison says so.

use std::collections::BTreeMap;

use render::assemble::{self, Kind, Occupant};
use render::codegen::Slot;
use render::colour::defaults::{self, Resolve};
use render::colour::field_ramp::{Compaction, FieldRamp, Lut, Map, Polarity, Ramp};
use validation::negative_control;

const CODE: &str = include_str!("data/field_ramps_before.txt");
const UNIFORMS: &str = include_str!("data/field_ramp_uniforms.txt");

fn sim_key(name: &str) -> Option<f64> {
    matches!(name, "eps_E" | "eps_L").then_some(1.0 / 1_048_576.0)
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

/// The fixture's code, by `<field> <colour | brightness | post>`.
fn before() -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut name = String::new();
    for line in CODE.lines() {
        if let Some(n) = line.strip_prefix("=== ") {
            name = n.to_owned();
        } else {
            let text: &mut String = out.entry(name.clone()).or_default();
            text.push_str(line);
            text.push('\n');
        }
    }
    out
}

/// The fixture's declared uniforms and input count, by the same names.
fn uniforms_before() -> BTreeMap<String, String> {
    UNIFORMS
        .lines()
        .filter_map(|l| l.split_once(": inputs "))
        .map(|(n, rest)| (n.to_owned(), rest.to_owned()))
        .collect()
}

/// A node's declared uniforms and input count, written as the fixture writes them: the bound ones, in order.
fn declared(kind: Kind, text: &str) -> String {
    let d = assemble::declaration(kind, &Occupant::Custom(text.to_owned()))
        .unwrap_or_else(|e| panic!("{e}"));
    let u: Vec<String> = d
        .uniforms
        .iter()
        .filter(|u| !u.is_baked())
        .map(|u| {
            format!(
                "{}:{}={:?}{}",
                u.name,
                u.ty.wgsl(),
                u.default,
                u.range.map(|r| format!(" {r:?}")).unwrap_or_default()
            )
        })
        .collect();
    format!("{}; {}", d.inputs.len(), u.join("; "))
}

/// Every scalar field's default ramp in `slot`, by field.
fn ramps(slot: Slot) -> Vec<(String, FieldRamp)> {
    let l = ledger::payload::ledger();
    let resolve = Resolve { sim_key: &sim_key };
    ledger::gen::validate(&l)
        .unwrap_or_else(|e| panic!("{e}"))
        .iter()
        .filter_map(|e| {
            defaults::field_ramp(e, slot, &resolve)
                .ok()
                .map(|r| (e.name.to_owned(), r))
        })
        .collect()
}

/// Every field's rows (its colour ramp, its brightness ramp and its post), made by `how` from the field's default
/// ramp, each held to the fixture.
fn check(how: &dyn Fn(&FieldRamp) -> FieldRamp) {
    let (code_before, uniforms) = (before(), uniforms_before());
    let mut rows = 0;
    for slot in [Slot::Colour, Slot::Brightness] {
        for (field, ramp) in ramps(slot) {
            let ramp = how(&ramp);
            let mut made = vec![(slot.kind().name(), slot.kind())];
            if slot == Slot::Brightness {
                made.push(("post", Kind::Post));
            }
            for (slot_name, kind) in made {
                let row = format!("{field} {slot_name}");
                let text = ramp.node_text(kind).unwrap_or_else(|e| panic!("{row}: {e}"));
                assert_eq!(
                    code(&text),
                    code_before[&row],
                    "{row}: the node's code is not the generated ramp's"
                );
                assert_eq!(
                    declared(kind, &text),
                    uniforms[&row],
                    "{row}: the node's declared uniforms are not the generated ramp's"
                );
                rows += 1;
            }
        }
    }
    assert_eq!(rows, code_before.len(), "every generated ramp is checked");
    println!("rows checked against the generated ramps: {rows}");
}

#[test]
fn every_default_ramp_is_the_node_as_it_was() {
    check(&|r| r.clone());
}

/// `ramp` with its field read as the next scalar field.
fn next_field(ramp: &FieldRamp) -> FieldRamp {
    let fields: Vec<String> = ramps(Slot::Colour).into_iter().map(|(f, _)| f).collect();
    let at = fields.iter().position(|f| f == ramp.field.field).unwrap_or(0);
    let next = &fields[(at + 1) % fields.len()];
    let mut out = ramp.clone();
    out.field = render::colour::field_ramp::ScalarField::payload(next).unwrap_or_else(|e| panic!("{e}"));
    out
}

negative_control!(
    every_default_ramp_is_the_node_as_it_was,
    "the node made as the next field",
    expected = "is not the generated ramp's",
    check(&next_field)
);

/// Whether `how` changes the code of every ramp's node in each slot it makes: a parameter that is not in the code
/// would draw the picture it did before.
fn check_changes(what: &str, how: &dyn Fn(&FieldRamp) -> FieldRamp) {
    let mut checked = 0;
    for slot in [Slot::Colour, Slot::Brightness] {
        for (field, ramp) in ramps(slot) {
            let kind = slot.kind();
            let (was, now) = (ramp.node_text(kind), how(&ramp).node_text(kind));
            let (was, now) = (
                was.unwrap_or_else(|e| panic!("{field}: {e}")),
                now.unwrap_or_else(|e| panic!("{field}: {e}")),
            );
            assert_ne!(
                code(&was),
                code(&now),
                "{field} {}: {what} did not change the code",
                kind.name()
            );
            checked += 1;
        }
    }
    println!("ramps checked: {checked}");
}

/// The curve every ramp is not on, as far as the code can tell: flag compacts to a different form from every other.
fn flag(r: &FieldRamp) -> FieldRamp {
    FieldRamp {
        compaction: Compaction::Flag,
        ..r.clone()
    }
}

fn other_polarity(r: &FieldRamp) -> FieldRamp {
    let flipped = match r.polarity {
        Polarity::High => Polarity::Low,
        Polarity::Low => Polarity::High,
    };
    r.clone().with_polarity(flipped)
}

/// A colour ramp on a lookup table it is not on; a lightness has no ramp, so it is left as it is.
fn other_ramp(r: &FieldRamp) -> FieldRamp {
    let table = match &r.map {
        Map::Ramp(m) if **m == Ramp::Lut(Lut::Magma) => Lut::Viridis,
        _ => Lut::Magma,
    };
    FieldRamp {
        map: match &r.map {
            Map::Ramp(_) => Map::ramp(Ramp::Lut(table)),
            Map::Lightness => Map::Lightness,
        },
        ..r.clone()
    }
}

#[test]
fn the_field_is_in_the_code() {
    check_changes("the next field", &next_field);
}

negative_control!(
    the_field_is_in_the_code,
    "a change that leaves the field as it is",
    expected = "did not change the code",
    check_changes("the same field", &|r| r.clone())
);

#[test]
fn the_curve_is_in_the_code() {
    check_changes("the flag curve", &|r| match r.compaction {
        Compaction::Flag => FieldRamp {
            compaction: Compaction::Log { eps: 1.0 },
            ..r.clone()
        },
        _ => flag(r),
    });
}

negative_control!(
    the_curve_is_in_the_code,
    "a change that leaves the curve as it is",
    expected = "did not change the code",
    check_changes("the same curve", &|r| r.clone())
);

#[test]
fn the_polarity_is_in_the_code() {
    check_changes("the other polarity", &other_polarity);
}

negative_control!(
    the_polarity_is_in_the_code,
    "a change that leaves the polarity as it is",
    expected = "did not change the code",
    check_changes("the same polarity", &|r| r.clone())
);

#[test]
fn the_ramp_is_in_the_code() {
    // A lightness has no ramp: only the colour slot's rows are held to change.
    let mut checked = 0;
    for (field, ramp) in ramps(Slot::Colour) {
        let (was, now) = (
            ramp.node_text(Kind::Colour).unwrap_or_else(|e| panic!("{e}")),
            other_ramp(&ramp)
                .node_text(Kind::Colour)
                .unwrap_or_else(|e| panic!("{e}")),
        );
        assert_ne!(code(&was), code(&now), "{field}: the ramp did not change the code");
        checked += 1;
    }
    println!("colour ramps checked: {checked}");
}

negative_control!(
    the_ramp_is_in_the_code,
    "a ramp left as it is",
    expected = "did not change the code",
    {
        let (field, ramp) = ramps(Slot::Colour).remove(0);
        let text = ramp.node_text(Kind::Colour).unwrap_or_else(|e| panic!("{e}"));
        assert_ne!(code(&text), code(&text), "{field}: the ramp did not change the code");
    }
);
