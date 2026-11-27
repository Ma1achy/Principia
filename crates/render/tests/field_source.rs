//! A read-side field as a source is one shipped file, `source/field`, with a `field` parameter. Pass criteria, written
//! before the runs:
//!
//! - For every readable field, the stain that holds `Occupant::Field(f)`, and the one that holds the shipped `field`
//!   source with its parameter written in as `f`, assemble to the code the field's own generated source assembled to
//!   (the oracle below is that generator as it was): the kept code, comments dropped, equal as text in the
//!   presentation and the debug view, and the fields the fragment reads equal. Code equal is picture equal: the source
//!   is the only node that differs.
//! - Control: the parameter taken as another field gives other code.
//! - The shipped file declares the one `field` parameter, and its out-port's field, vector or scalar, follows it.

use render::assemble::{self, params, Kind, Node, Occupant, Stain, Tier, View};
use render::registry::{self, ZERO};
use validation::negative_control;

/// The text a field's source was generated as before it was one shipped file: the oracle.
fn oracle(f: &str) -> String {
    let l = ledger::payload::ledger();
    let entries = ledger::gen::validate(&l).unwrap_or_else(|e| panic!("{e}"));
    let ty = ledger::gen::read::members(&l.words, &entries)
        .into_iter()
        .find(|m| m.name == f)
        .map(|m| m.wgsl)
        .unwrap_or_else(|| panic!("no field {f}"));
    if ty == "vec3<f32>" {
        return format!("fn source(ctx: Ctx) -> Field {{ return Field(ctx.sample.{f}, 0.0); }}\n");
    }
    let value = match ty {
        "bool" => format!("select(0.0, 1.0, ctx.sample.{f})"),
        _ => format!("f32(ctx.sample.{f})"),
    };
    format!("fn source(ctx: Ctx) -> Field {{ return Field({value}, 0.0, 0.0, 0.0); }}\n")
}

fn node(kind: Kind, occupant: Occupant, inputs: &[Option<usize>]) -> Node {
    Node {
        kind,
        occupant,
        inputs: inputs.to_vec(),
    }
}

/// The source `occupant` shown by a colour that reads its field, through the pass-through combiner.
fn showing(occupant: Occupant) -> Stain {
    Stain::new(vec![
        node(Kind::Source, occupant, &[]),
        node(
            Kind::Colour,
            Occupant::Custom(
                "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.inputs[0].x); }".into(),
            ),
            &[Some(0)],
        ),
        node(
            Kind::Combiner,
            Occupant::BuiltIn("pass_through".into()),
            &[Some(1), None],
        ),
        node(Kind::Out, Occupant::None, &[Some(2)]),
    ])
    .unwrap_or_else(|e| panic!("{e}"))
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

/// The kept code and the fields read, in both views, of the stain showing `occupant`.
fn assembled(occupant: Occupant) -> Vec<(String, Vec<String>)> {
    let stain = showing(occupant);
    [View::Presentation, View::Debug]
        .into_iter()
        .map(|v| {
            let f =
                assemble::assemble_kept(&stain, Tier::FULL, v).unwrap_or_else(|e| panic!("{e}"));
            (code(&f.source), f.fields.clone())
        })
        .collect()
}

/// The shipped `field` source with its parameter written in as `f`.
fn shipped(f: &str) -> Occupant {
    let e = registry::named(Kind::Source, "field").unwrap_or_else(|e| panic!("{e}"));
    let d = assemble::declaration(Kind::Source, &Occupant::BuiltIn("field".into()))
        .unwrap_or_else(|e| panic!("{e}"));
    let u = d
        .uniforms
        .iter()
        .find(|u| u.ty == assemble::UniformType::Field)
        .expect("the shipped source declares a `field` parameter");
    let at = assemble::source_fields()
        .unwrap_or_else(|e| panic!("{e}"))
        .iter()
        .position(|n| n == f)
        .expect("a field");
    let text = params::specialise(&e.source, &[u], &|_| vec![at as f64]).unwrap_or_else(|e| panic!("{e}"));
    Occupant::Custom(text)
}

/// Whether each field's stain, written as `how` says, assembles as its oracle does.
fn check(how: &dyn Fn(&str, &[String]) -> Occupant) {
    let names = assemble::source_fields().unwrap_or_else(|e| panic!("{e}"));
    assert!(names.len() > 20, "the readable fields: {}", names.len());
    for (k, f) in names.iter().enumerate() {
        let want = assembled(Occupant::Custom(oracle(f)));
        let got = assembled(how(f, &names));
        assert_eq!(got, want, "field `{f}` (number {k}) assembles as it did");
    }
}

#[test]
fn every_field_assembles_as_before() {
    check(&|f, _| Occupant::Field(f.to_owned()));
}

#[test]
fn the_shipped_source_with_each_field_assembles_as_before() {
    check(&|f, _| shipped(f));
}

negative_control!(
    the_shipped_source_with_each_field_assembles_as_before,
    "the parameter written in as the next field",
    expected = "assembles as it did",
    check(&|f, names| {
        let at = names.iter().position(|n| n == f).unwrap_or(0);
        shipped(&names[(at + 1) % names.len()])
    })
);

negative_control!(
    every_field_assembles_as_before,
    "the parameter taken as the next field gives other code",
    expected = "assembles as it did",
    check(&|f, names| {
        let at = names.iter().position(|n| n == f).unwrap_or(0);
        Occupant::Field(names[(at + 1) % names.len()].clone())
    })
);

/// The declaration of `source` is one `field` parameter and nothing else, and takes no input.
fn check_one_field_parameter(source: &str) {
    let d = assemble::Declaration::parse(source).unwrap_or_else(|e| panic!("{e}"));
    let fields: Vec<_> = d
        .uniforms
        .iter()
        .filter(|u| u.ty == assemble::UniformType::Field)
        .collect();
    assert_eq!(fields.len(), 1, "one field parameter");
    assert_eq!(d.uniforms.len(), 1, "and nothing else");
    assert!(d.inputs.is_empty(), "a source takes no input");
}

#[test]
fn the_shipped_field_source_declares_one_field_parameter() {
    let e = registry::named(Kind::Source, "field").unwrap_or_else(|e| panic!("{e}"));
    check_one_field_parameter(&e.source);
    // The zero source is another shipped source of the slot: this one is in the library beside it.
    assert!(registry::named(Kind::Source, ZERO).is_ok());
}

negative_control!(
    the_shipped_field_source_declares_one_field_parameter,
    "a second uniform beside the field parameter",
    expected = "and nothing else",
    {
        let e = registry::named(Kind::Source, "field").unwrap_or_else(|e| panic!("{e}"));
        check_one_field_parameter(&format!("// @uniform gain: f32 = 1.0\n{}", e.source));
    }
);

/// Each field's `Occupant::Field` declares its field as the parameter's default, so an editor shows it chosen.
fn check_default(shift: usize) {
    let names = assemble::source_fields().unwrap_or_else(|e| panic!("{e}"));
    for (k, f) in names.iter().enumerate() {
        let d = assemble::declaration(Kind::Source, &Occupant::Field(f.clone()))
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            d.uniforms[0].default,
            [((k + shift) % names.len()) as f64],
            "`{f}`'s default"
        );
    }
}

#[test]
fn a_field_occupant_declares_its_field_as_the_default() {
    check_default(0);
}

negative_control!(
    a_field_occupant_declares_its_field_as_the_default,
    "the default taken as the next field",
    expected = "'s default",
    check_default(1)
);
