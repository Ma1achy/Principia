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
//! - Every row's scene builds on the CPU, and the rows whose field no setter writes are the ones `UNWRITTEN` lists:
//!   those are drawn on the states and the fresh sample only.
//! - On a GPU (where there is an adapter): for every row, the picture of the shipped node made as the field's default
//!   ramp equals, bit for bit in all four channels of every pixel, the picture of the generated text it replaces,
//!   over a row of samples holding the field's NaN, both infinities, the −1 sentinel, zero, a tiny and a huge value,
//!   a decode_failed, a sim_failed and two reserved states, a pixel the chart forbids, and the fresh sample's unset
//!   d_min. Zero pixels differ. Control: the node of the next field differs in some pixel.
//! - The ramps the golden scenes and the checked-in runtime copies draw, which are not the defaults (`d_min` and
//!   `length` by their own constructors, every other field on `lin` over [0, 1] on viridis), are the node as the
//!   generator wrote them (`tests/data/field_ramp_specs_before.txt`, 42 fields): code and bound uniforms equal.
//! - Every setting of the node assembles through the one assembler on the CPU: each curve, each end of a `lin` range as
//!   a value, measured or the horizon, symmetric or not, each ramp and the node's gradient, either polarity, as a colour
//!   and as a lightness with its post, on the FTLE, a field with a drift's band, and `d_min`. A gradient's
//!   default is the ramp's own stops. Control: a ramp calling a function that does not exist does not assemble.
//! - Controls: the node taken as another field, another curve, the other polarity or another ramp is another code, and
//!   the comparison says so.

use std::collections::BTreeMap;

use ledger::gen::numeric::End;
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

/// Whether `how` changes the code of every colour ramp's node: a lightness has no ramp, so only the colour slot.
fn check_ramp_changes(how: &dyn Fn(&FieldRamp) -> FieldRamp) {
    let mut checked = 0;
    for (field, ramp) in ramps(Slot::Colour) {
        let (was, now) = (
            ramp.node_text(Kind::Colour)
                .unwrap_or_else(|e| panic!("{e}")),
            how(&ramp)
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

#[test]
fn the_ramp_is_in_the_code() {
    check_ramp_changes(&other_ramp);
}

negative_control!(
    the_ramp_is_in_the_code,
    "a change that leaves the ramp as it is",
    expected = "did not change the code",
    check_ramp_changes(&|r| r.clone())
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

/// The row of samples for `field`: [`VALUES`] written to it where a setter of the sample's record writes it, then a
/// `decode_failed`, a `sim_failed` and the two reserved states, a pixel the chart forbids, and a fresh sample (its
/// `d_min` unset). The flag says whether the field's own values were written: a field no setter writes is drawn on the
/// states and the fresh sample only.
fn scene_for(field: &str) -> (validation::golden_scene::Scene, bool) {
    use render::colour::outcome::code;
    let mut scene = row_scene("field_ramp_node", SAMPLES, Colouring::View("state"))
        .unwrap_or_else(|e| panic!("{e}"));
    let mut written = true;
    for (i, v) in (0u32..).zip(VALUES) {
        let bits = v as u32; // saturating: NaN is 0, +∞ is the greatest, a negative is 0
                             // `d_min` is stored as an f16, and its setter refuses NaN and a negative value: those are the f16's own bits.
        if field == "d_min" && (v.is_nan() || v < 0.0) {
            let raw: u32 = if v.is_nan() {
                0x7e00
            } else if v == f32::NEG_INFINITY {
                0xfc00
            } else {
                0xbc00
            };
            let w = scene.set.simstate(i).packed_a;
            scene.set.sample(i).packed_a((w & 0xffff) | (raw << 16));
            continue;
        }
        match field {
            "m0" => scene.set.ic(i).m0 = v,
            "m1" => scene.set.ic(i).m1 = v,
            "m2" => scene.set.ic(i).m2 = v,
            "q_mass" => scene.set.ic(i).q_mass = v,
            "rho_mag" => scene.set.ic(i).rho_mag = v,
            "lambda_mag" => scene.set.ic(i).lambda_mag = v,
            "rho_ratio" => scene.set.ic(i).rho_ratio = v,
            "rho_angle" => scene.set.ic(i).rho_angle = v,
            "K_0" => scene.set.ic(i).K_0 = v,
            "V_0" => scene.set.ic(i).V_0 = v,
            "virial_ratio" => scene.set.ic(i).virial_ratio = v,
            "r_min_pair_0" => scene.set.ic(i).r_min_pair_0 = v,
            _ => {
                let mut sample = scene.set.sample(i);
                match field {
                    "S" => {
                        sample.S(v);
                    }
                    "theta" => {
                        sample.theta(v);
                    }
                    "mean_y" => {
                        sample.mean_y(v);
                    }
                    "C_ty" => {
                        sample.C_ty(v);
                    }
                    "E_0" => {
                        sample.E_0(v);
                    }
                    "Lz_0" => {
                        sample.Lz_0(v);
                    }
                    "closure_min" => {
                        sample.closure_min(v);
                    }
                    "d_min" => {
                        sample.d_min(v);
                    }
                    "dE_max" => {
                        sample.drift_max(v, 0.0);
                    }
                    "dLz_max" => {
                        sample.drift_max(0.0, v);
                    }
                    "dE_reg_max" => {
                        sample.reg_drift(0.0, v);
                    }
                    "dE_reg_final" => {
                        sample.reg_drift(v, 0.0);
                    }
                    "total_substeps" => {
                        sample.total_substeps(bits);
                    }
                    "closure_step" => {
                        sample.closure_step(bits.min(u32::from(u16::MAX)) as u16);
                    }
                    "coll_floor" => {
                        sample.coll_floor(bits);
                    }
                    "dmin_pair" => {
                        sample.dmin_pair(bits);
                    }
                    "detail" => {
                        sample.detail(bits);
                    }
                    "t_end_step" => {
                        sample.times(bits, 0);
                    }
                    "t_dmin_step" => {
                        sample.times(0, bits);
                    }
                    "floor_hits" => {
                        sample.budget(false, bits, 0);
                    }
                    "cap_hits" => {
                        sample.budget(false, 0, bits);
                    }
                    "length" => {
                        sample.word([0; 4], bits.min(127));
                    }
                    "payload" => {
                        sample.word([bits, 0, 0, 0], 1);
                    }
                    _ => written = false,
                }
            }
        }
    }
    for (i, state) in [
        (8u32, code("decode_failed")),
        (9, code("sim_failed")),
        (10, 6),
        (11, 7),
    ] {
        scene.set.sample(i).state(state);
    }
    scene.set.set_forbidden(12, true);
    (scene, written)
}

/// The picture of `ramp` (and `post`) over [`scene_for`] `field`, as every pixel's four channels.
fn picture(
    h: &GpuHarness,
    field: &str,
    kind: Kind,
    ramp: &str,
    post: Option<&str>,
) -> Vec<[u32; 4]> {
    let (scene, _) = scene_for(field);
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

/// Every row's scene builds, here, and the fields no setter writes are the ones listed: those rows are drawn on the
/// states and the fresh sample only.
fn check_scenes(listed: &[&str]) {
    let mut unwritten: Vec<String> = ramps(Slot::Colour)
        .into_iter()
        .map(|(f, _)| f)
        .filter(|f| !scene_for(f).1)
        .collect();
    unwritten.sort();
    let mut want: Vec<String> = listed.iter().map(|s| (*s).to_owned()).collect();
    want.sort();
    assert_eq!(unwritten, want, "the fields drawn without their values");
}

/// The fields derived from other members when read (the FTLE from `S` and the step count, a drift from the energies, the
/// diffusion from its accumulators) or packed in a shared word (`I_max`): no setter writes them.
const UNWRITTEN: [&str; 5] = ["I_max", "Lz_drift", "diffusion", "energy_drift", "ftle"];

#[test]
fn every_scene_writes_the_values_of_its_field() {
    check_scenes(&UNWRITTEN);
}

negative_control!(
    every_scene_writes_the_values_of_its_field,
    "a field listed as unwritten that is written",
    expected = "the fields drawn without their values",
    check_scenes(&[
        "I_max",
        "Lz_drift",
        "diffusion",
        "energy_drift",
        "ftle",
        "S"
    ])
);

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

// ── The specs the golden scenes and the runtime copies use ────────────────────────────────────────────────────────

const SPECS: &str = include_str!("data/field_ramp_specs_before.txt");

/// The ramp of `field` as the runtime copies and the golden scenes have it: `d_min`'s and `length`'s by their own
/// constructors, every other `lin` over [0, 1] on viridis.
fn spec(field: &str) -> FieldRamp {
    match field {
        "d_min" => FieldRamp::d_min(),
        "length" => FieldRamp::length(),
        _ => render::colour::field_ramp::ScalarField::payload(field).map(|f| {
            FieldRamp::new(
                f,
                Compaction::Lin { lo: 0.0, hi: 1.0 },
                Map::ramp(Ramp::Lut(Lut::Viridis)),
            )
        }),
    }
    .unwrap_or_else(|e| panic!("{field}: {e}"))
}

fn check_specs(how: &dyn Fn(&FieldRamp) -> FieldRamp) {
    let mut rows = 0;
    let mut name = String::new();
    let mut blocks: BTreeMap<String, String> = BTreeMap::new();
    for line in SPECS.lines() {
        match line.strip_prefix("=== ") {
            Some(n) => name = n.to_owned(),
            None => {
                let t = blocks.entry(name.clone()).or_default();
                t.push_str(line);
                t.push('\n');
            }
        }
    }
    for (field, old) in &blocks {
        let text = how(&spec(field))
            .node_text(Kind::Colour)
            .unwrap_or_else(|e| panic!("{field}: {e}"));
        assert_eq!(
            code(&text),
            code(old),
            "{field}: the node's code is not the generated ramp's"
        );
        assert_eq!(
            declared(Kind::Colour, &text),
            declared(Kind::Colour, old),
            "{field}: the node's declared uniforms are not the generated ramp's"
        );
        rows += 1;
    }
    assert_eq!(rows, 42, "every field a ramp reads is checked");
    println!("specs checked against the generated ramps: {rows}");
}

#[test]
fn the_other_specs_are_the_node_as_they_were() {
    check_specs(&|r| r.clone());
}

negative_control!(
    the_other_specs_are_the_node_as_they_were,
    "every ramp made on the low polarity",
    expected = "is not the generated ramp's",
    check_specs(&other_polarity)
);

// ── Every setting of the node assembles ───────────────────────────────────────────────────────────────────────

/// The node made as every curve (each end of a `lin` range read as a value, measured or the horizon, symmetric or not),
/// every ramp (each lookup table, and `lerp` and `diverging` as the node's gradient) and either polarity, in the colour
/// slot, and as a lightness with its post, on a field with no band or unset value, one with a drift's band and `d_min`,
/// whose unset value draws "not yet": each assembles through the one assembler, its text first changed by `edit`.
/// A gradient's default is the ramp's own stops.
fn check_every_setting(edit: &dyn Fn(String) -> String) {
    use render::assemble::{self, Declaration, Tier};
    use render::colour::field_ramp::ScalarField;
    let measured = End::Measured;
    let curves = [
        Compaction::Lin { lo: 0.0, hi: 1.0 },
        Compaction::LinRead {
            lo: measured,
            hi: measured,
            symmetric: false,
        },
        Compaction::LinRead {
            lo: measured,
            hi: End::Fixed(2.0),
            symmetric: false,
        },
        Compaction::LinRead {
            lo: End::Fixed(0.0),
            hi: measured,
            symmetric: false,
        },
        Compaction::LinRead {
            lo: End::Fixed(0.0),
            hi: End::Horizon,
            symmetric: false,
        },
        Compaction::LinRead {
            lo: End::Fixed(-3.0),
            hi: End::Fixed(5.0),
            symmetric: true,
        },
        Compaction::LinRead {
            lo: measured,
            hi: measured,
            symmetric: true,
        },
        Compaction::Log { eps: 0.5 },
        Compaction::Cyclic { period: 2.0 },
        Compaction::Diverging { x0: 0.25 },
        Compaction::Flag,
    ];
    let (c0, c1, c2) = ([0.9, 0.1, 0.0], [0.0, 0.2, 0.8], [0.5, 0.5, 0.5]);
    let ramps = [
        Ramp::Lut(Lut::Viridis),
        Ramp::Lut(Lut::Magma),
        Ramp::Lut(Lut::Twilight),
        Ramp::Lut(Lut::Coolwarm),
        Ramp::Lut(Lut::Grey),
        Ramp::Lerp { c0, c1 },
        Ramp::Diverging {
            lo: c0,
            mid: c2,
            hi: c1,
        },
    ];
    let mut made = 0;
    // Every curve on the FTLE; the curves that meet a drift's band and `d_min`'s unset value on those fields.
    let (lin, log, diverging, flag) = (curves[0], curves[7], curves[9], curves[10]);
    let on: [(&str, Vec<Compaction>); 3] = [
        ("ftle", curves.to_vec()),
        ("energy_drift", vec![lin, diverging]),
        ("d_min", vec![lin, log, flag]),
    ];
    for (field, curves) in on {
        let f = ScalarField::payload(field).unwrap_or_else(|e| panic!("{e}"));
        for c in curves {
            for p in [Polarity::High, Polarity::Low] {
                let mut maps: Vec<Map> = ramps.iter().map(|r| Map::ramp(*r)).collect();
                maps.push(Map::Lightness);
                for map in maps {
                    let ramp = FieldRamp::new(f.clone(), c, map.clone()).with_polarity(p);
                    let kind = ramp.slot().kind();
                    let text = ramp.node_text(kind).unwrap_or_else(|e| panic!("{e}"));
                    if let Map::Ramp(r) = &map {
                        if let Some(stops) = r.as_gradient() {
                            let d = Declaration::parse(&text).unwrap_or_else(|e| panic!("{e}"));
                            let u = d.uniforms.iter().find(|u| u.name == "colours");
                            let held: Vec<f32> =
                                u.map_or(vec![], |u| u.default.iter().map(|&x| x as f32).collect());
                            let want: Vec<f32> = stops.iter().map(|&x| x as f32).collect();
                            assert_eq!(
                                held,
                                want,
                                "{field} {}: the gradient's default is not the ramp's stops",
                                c.name()
                            );
                        }
                    }
                    let post = (kind == Kind::Brightness)
                        .then(|| ramp.node_text(Kind::Post).unwrap_or_else(|e| panic!("{e}")));
                    let stain = drawn(kind, &edit(text), post.as_deref());
                    assemble::assemble(&stain, Tier::FULL).unwrap_or_else(|e| {
                        panic!(
                            "{field} · {} · {p:?} · {map:?} did not assemble: {e}",
                            c.name()
                        )
                    });
                    made += 1;
                }
            }
        }
    }
    println!("settings assembled: {made}");
}

#[test]
fn every_setting_of_the_node_assembles() {
    check_every_setting(&|t| t);
}

negative_control!(
    every_setting_of_the_node_assembles,
    "a ramp calling a library function that does not exist does not assemble",
    expected = "did not assemble",
    check_every_setting(&|t| t.replace("ramp_", "ramp__"))
);
