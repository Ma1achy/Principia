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
//! - On a GPU (where there is an adapter): for every row, the picture of the shipped node made as the field's default
//!   ramp equals, bit for bit in all four channels of every pixel, the picture of the generated text it replaces,
//!   over a row of samples holding the field's NaN, both infinities, the −1 sentinel, zero, a tiny and a huge value,
//!   a decode_failed, a sim_failed and two reserved states, a pixel the chart forbids, and the fresh sample's unset
//!   d_min. Zero pixels differ. Control: the node of the next field differs in some pixel.
//! - Controls: the node taken as another field, another curve, the other polarity or another ramp is another code, and
//!   the comparison says so.

use std::collections::BTreeMap;

use render::assemble::{self, Kind, Occupant};
use render::codegen::Slot;
use render::colour::defaults::{self, Resolve};
use render::colour::field_ramp::{Compaction, FieldRamp, Lut, Map, Polarity, Ramp};
use render::registry::ZERO_SOURCE;
use validation::golden_scene::{render_stain, row_scene, Colouring};
use validation::gpu::GpuHarness;
use validation::negative_control;

/// The generated ramps' code, one block a row, with the `// @uniform` lines they declared (`code` leaves them out).
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
                let text = ramp
                    .node_text(kind)
                    .unwrap_or_else(|e| panic!("{row}: {e}"));
                assert_eq!(
                    code(&text),
                    code(&code_before[&row]),
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
    let at = fields
        .iter()
        .position(|f| f == ramp.field.field)
        .unwrap_or(0);
    let next = &fields[(at + 1) % fields.len()];
    let mut out = ramp.clone();
    out.field =
        render::colour::field_ramp::ScalarField::payload(next).unwrap_or_else(|e| panic!("{e}"));
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
            ramp.node_text(Kind::Colour)
                .unwrap_or_else(|e| panic!("{e}")),
            other_ramp(&ramp)
                .node_text(Kind::Colour)
                .unwrap_or_else(|e| panic!("{e}")),
        );
        assert_ne!(
            code(&was),
            code(&now),
            "{field}: the ramp did not change the code"
        );
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
        let text = ramp
            .node_text(Kind::Colour)
            .unwrap_or_else(|e| panic!("{e}"));
        assert_ne!(
            code(&text),
            code(&text),
            "{field}: the ramp did not change the code"
        );
    }
);

// ── Pictures ──────────────────────────────────────────────────────────────────────────────────────────────────

/// The samples a row holds: each value, then the states a pixel can be in whatever its value.
const VALUES: [f32; 8] = [
    f32::NAN,
    f32::INFINITY,
    f32::NEG_INFINITY,
    -1.0,
    0.0,
    1e-30,
    0.5,
    1e30,
];
const SAMPLES: u32 = 14;

/// The stain of the zero source into `colour` and `brightness` through Replace-L, with the post before `OUT`.
fn drawn(kind: Kind, ramp: &str, post: Option<&str>) -> render::assemble::Stain {
    use render::assemble::Node;
    let node = |kind, occupant, inputs: Vec<Option<usize>>| Node {
        kind,
        occupant,
        inputs,
    };
    let (colour, brightness) = match kind {
        Kind::Colour => (Occupant::Custom(ramp.to_owned()), Occupant::None),
        _ => (Occupant::None, Occupant::Custom(ramp.to_owned())),
    };
    let mut nodes = vec![
        node(
            Kind::Source,
            Occupant::Custom(ZERO_SOURCE.to_owned()),
            vec![],
        ),
        node(Kind::Colour, colour, vec![Some(0)]),
        node(Kind::Brightness, brightness, vec![Some(0)]),
        node(
            Kind::Combiner,
            Occupant::BuiltIn("replace_l".into()),
            vec![Some(1), Some(2)],
        ),
    ];
    let last = match post {
        Some(p) => {
            nodes.push(node(
                Kind::Post,
                Occupant::Custom(p.to_owned()),
                vec![Some(3)],
            ));
            4
        }
        None => 3,
    };
    nodes.push(node(Kind::Out, Occupant::None, vec![Some(last)]));
    render::assemble::Stain::new(nodes).unwrap_or_else(|e| panic!("{e}"))
}

/// The picture of `ramp` (and `post`) over the row of [`VALUES`] written to `field`, as every pixel's four channels.
fn picture(
    h: &GpuHarness,
    field: &str,
    kind: Kind,
    ramp: &str,
    post: Option<&str>,
) -> Vec<[u32; 4]> {
    let mut scene = row_scene("field_ramp_node", SAMPLES, Colouring::View("state"))
        .unwrap_or_else(|e| panic!("{e}"));
    for (i, v) in (0u32..).zip(VALUES) {
        // The fields a sample's setters write; any other is left as a fresh sample has it, and is drawn by the states
        // below.
        let mut sample = scene.set.sample(i);
        match field {
            "S" => sample.S(v),
            "theta" => sample.theta(v),
            "mean_y" => sample.mean_y(v),
            "C_ty" => sample.C_ty(v),
            "E_0" => sample.E_0(v),
            "Lz_0" => sample.Lz_0(v),
            "closure_min" => sample.closure_min(v),
            "d_min" => sample.d_min(v),
            "dE_max" => sample.drift_max(v, 0.0),
            "dLz_max" => sample.drift_max(0.0, v),
            "dE_reg_max" => sample.reg_drift(0.0, v),
            _ => &mut sample,
        };
    }
    for (i, state) in [(8u32, 7u32), (9, 1), (10, 6), (11, 7)] {
        scene.set.sample(i).state(state);
    }
    scene.set.set_forbidden(12, true);
    let image = render_stain(
        h.device(),
        h.queue(),
        &drawn(kind, ramp, post),
        &scene.set,
        &scene.context,
        &|_, name| (name == "u_range").then(|| vec![-1.0, 1.0]),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    image.iter().map(|p| p.map(f32::to_bits)).collect()
}

/// Whether, for every row, the node's picture is the generated text's, bit for bit; `how` makes the ramp the node is.
fn check_pictures(how: &dyn Fn(&FieldRamp) -> FieldRamp) {
    let h = GpuHarness::new().expect("a GPU device");
    let old = before();
    let mut rows = 0;
    for slot in [Slot::Colour, Slot::Brightness] {
        for (field, ramp) in ramps(slot) {
            let kind = slot.kind();
            let node = how(&ramp);
            let was = &old[&format!("{field} {}", kind.name())];
            let now = node.node_text(kind).unwrap_or_else(|e| panic!("{e}"));
            let post = (slot == Slot::Brightness).then(|| {
                (
                    old[&format!("{field} post")].clone(),
                    node.node_text(Kind::Post).unwrap_or_else(|e| panic!("{e}")),
                )
            });
            let a = picture(&h, &field, kind, was, post.as_ref().map(|p| p.0.as_str()));
            let b = picture(&h, &field, kind, &now, post.as_ref().map(|p| p.1.as_str()));
            let n = a.iter().zip(&b).filter(|(x, y)| x != y).count();
            assert_eq!(
                n,
                0,
                "{field} {}: {n} pixel(s) differ from the generated ramp's",
                kind.name()
            );
            rows += 1;
        }
    }
    println!("rows drawn, none with a differing pixel: {rows}");
}

#[test]
fn every_ramp_draws_the_picture_it_drew() {
    check_pictures(&|r| r.clone());
}

negative_control!(
    every_ramp_draws_the_picture_it_drew,
    "the node made as the next field",
    expected = "pixel(s) differ from the generated ramp's",
    check_pictures(&next_field)
);
