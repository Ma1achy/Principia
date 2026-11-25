//! A node's richer parameters in the engine (`render::assemble::params`, grammar version 2): a gradient, a list, a lut,
//! a field and an image are values of a node's params, and
//!
//! - round-trip through the canonical form and a saved stain (`each_type_round_trips_*`);
//! - are refused where they are no value of their declaration (`bad_values_are_refused`);
//! - the custom-stop ramp is a gradient parameter, a ramp saved before still opens, and a ramp reads the same colours as
//!   its gradient (`the_custom_stop_ramp_*`);
//! - a pair stays ordered (`a_pair_stays_ordered`), a field is read in the code (`a_field_parameter_*`), a wire an input
//!   does not take is noted and still attaches (`a_wire_an_input_does_not_take_*`), and a description carries what a
//!   source declares (`the_description_carries_*`).
//!
//! The engine's own tests have no render probes, so each control arm here mutates the case: it plants the wrong value
//! the property says is refused, or the change it says is no change, and the same check must fail.

use engine::contract::stain::describe::{describe, standing, Matters};
use engine::contract::stain::graph::{NodeId, NodeKind, Occupant, StainGraph};
use engine::contract::stain::ramp::{Ramp, Stop};
use render::assemble::{self, params, Subtype as FieldSubtype};
use validation::negative_control;

const COMBINER: NodeId = NodeId(0);

/// A colour node reading its input as the lut's argument through a gradient, with a list, a field and an image.
const EVERY_TYPE: &str = "// @version 2
// @uniform ramp: gradient<8> = oklab 0:(0.25, 0.0, 0.0) 1:(0.875, 0.0, 0.0)
// @control ramp gradient \"Ramp\"
// @uniform sites: list<vec3<f32>, 4> = [(1.0, 0.0, 0.0), (0.0, 1.0, 0.0)]
// @control sites list \"Sites\"
// @uniform curve: lut<8> = [0.0, 0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 1.0]
// @control curve lut \"Curve\"
// @uniform source: field = d_min
// @control source field \"Field\"
// @uniform picture: image = none
// @control picture image \"Picture\"
fn colour(ctx: Ctx) -> vec3<f32> {
    let t = lut8_at(uniforms.curve, uniforms.source);
    let a = gradient8_at(uniforms.ramp, t);
    return a + uniforms.sites.items[0u].xyz * f32(uniforms.sites.count) * f32(uniforms.picture);
}
";

fn field_of(want: FieldSubtype) -> String {
    assemble::source_fields()
        .expect("the fields")
        .into_iter()
        .find(|f| assemble::field_subtype(f).ok() == Some(want))
        .expect("a field of the subtype")
}

fn field_index(name: &str) -> f64 {
    assemble::source_fields()
        .expect("the fields")
        .iter()
        .position(|f| f == name)
        .expect("a field") as f64
}

/// A source, then a colour of `text` fed by it, wired to the combiner.
fn graph(text: &str) -> (StainGraph, NodeId) {
    let mut g = StainGraph::new();
    let s = g
        .add(
            NodeKind::Source,
            Occupant::Field(field_of(FieldSubtype::Scalar)),
        )
        .expect("a source");
    let c = g
        .add(NodeKind::Colour, Occupant::Custom(text.to_owned()))
        .expect("a colour");
    g.connect(s, c, 0).expect("source to colour");
    g.connect(c, COMBINER, 0).expect("colour to combiner");
    (g, c)
}

fn values() -> Vec<(&'static str, Vec<f64>)> {
    vec![
        (
            "ramp",
            vec![
                1.0, 1.0, 0.25, 0.0, -0.5, 0.0, 0.5, 0.125, 0.25, 0.5, 0.75, 0.0, 0.5, 1.0,
            ],
        ),
        ("sites", vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]),
        ("curve", vec![0.0, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 1.0]),
        ("source", vec![field_index("ftle")]),
        ("picture", vec![7.0]),
    ]
}

fn set_all(g: &mut StainGraph, c: NodeId) {
    for (name, v) in values() {
        g.set_param(c, name, v.clone())
            .unwrap_or_else(|e| panic!("`{name}` = {v:?}: {e}"));
    }
}

fn check_round_trip(damage: bool) {
    // Pass criterion: a graph holding each type's value, saved and read back, is the same graph: its params are the
    // values, and its canonical form's render key, which holds the params, is the same.
    let (mut g, c) = graph(EVERY_TYPE);
    set_all(&mut g, c);
    let text = serde_json::to_string(&g).expect("saved");
    let mut back: StainGraph = serde_json::from_str(&text).expect("read back");
    if damage {
        // A gradient that lost its last stop on the way.
        let mut v = values()[0].1.clone();
        v.truncate(v.len() - 4);
        back.set_param(c, "ramp", v).expect("two stops");
    }
    for (name, v) in values() {
        assert_eq!(
            back.node(c).expect("c").params.get(name),
            Some(&v),
            "`{name}` round-tripped"
        );
    }
    assert_eq!(
        back.canonical().expect("form").render_key(),
        g.canonical().expect("form").render_key(),
        "the canonical form round-tripped"
    );
    let form = g.canonical().expect("form");
    let params = &form
        .nodes
        .iter()
        .find(|n| n.occupant != Occupant::None && n.kind == NodeKind::Colour)
        .expect("the colour")
        .params;
    assert_eq!(
        params["picture"],
        [7.0],
        "the canonical form holds the value"
    );
}

#[test]
fn each_type_round_trips_through_the_canonical_form_and_a_saved_stain() {
    check_round_trip(false);
}

negative_control!(
    each_type_round_trips_through_the_canonical_form_and_a_saved_stain,
    "a gradient that lost its last stop on the way is not the one saved",
    expected = "round-tripped",
    check_round_trip(true)
);

fn check_refusals(plant: bool) {
    // Pass criterion: each value below is no value of its declaration, so `set_param` refuses it.
    let (g, c) = graph(EVERY_TYPE);
    let mut bad: Vec<(&str, Vec<f64>)> = vec![
        ("ramp", vec![1.0, 0.0, 0.5, 0.0, 0.0, 0.0]),
        (
            "ramp",
            vec![1.0, 0.0, 0.5, 0.0, 0.0, 0.8, 0.5, 0.0, 0.0, 0.2],
        ),
        (
            "ramp",
            vec![2.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.5, 0.0, 0.0, 1.0],
        ),
        ("sites", vec![1.0, 2.0]),
        ("sites", vec![0.0; 15]),
        ("curve", vec![0.0; 7]),
        ("source", vec![1e9]),
        ("source", vec![-1.0]),
        ("picture", vec![0.5]),
        ("picture", vec![-1.0]),
    ];
    if plant {
        // A value that is one: a two-stop gradient in order.
        bad.push((
            "ramp",
            vec![1.0, 0.0, 0.25, 0.0, 0.0, 0.0, 0.875, 0.0, 0.0, 1.0],
        ));
    }
    for (name, v) in bad {
        let mut h = g.clone();
        assert!(
            h.set_param(c, name, v.clone()).is_err(),
            "`{name}` = {v:?} was accepted"
        );
    }
}

#[test]
fn bad_values_are_refused() {
    check_refusals(false);
}

negative_control!(
    bad_values_are_refused,
    "a good gradient among the bad is accepted, so the check fails",
    expected = "was accepted",
    check_refusals(true)
);

/// A colour node declaring a gradient, for the ramp's home, and the ramp the editor holds.
const RAMPED: &str = "// @version 2
// @uniform ramp: gradient<8> = oklab 0:(0.25, 0.0, 0.0) 1:(0.875, 0.0, 0.0)
fn colour(ctx: Ctx) -> vec3<f32> { return gradient8_at(uniforms.ramp, ctx.inputs[0].x); }
";

fn a_ramp(cyclic: bool) -> Ramp {
    Ramp::new(
        vec![
            Stop {
                position: 0.0,
                oklab: [0.3, 0.1, -0.1],
            },
            Stop {
                position: 0.25,
                oklab: [0.6, -0.12, 0.05],
            },
            Stop {
                position: 0.75,
                oklab: [0.9, 0.02, 0.15],
            },
        ],
        cyclic,
    )
    .expect("a ramp")
}

fn check_ramp(lose_cyclic: bool) {
    // Pass criterion: a ramp set on a node that declares a gradient is that gradient's value, and reads back as the
    // same ramp, cyclic or not; the CPU reads the colours of the ramp and of its gradient within 1e-5 apart (`f32`
    // against `f64`).
    for cyclic in [false, true] {
        let ramp = a_ramp(cyclic);
        let (mut g, c) = graph(RAMPED);
        g.set_ramp(c, Some(ramp.clone())).expect("set");
        assert_eq!(
            g.node(c).expect("c").ramp,
            None,
            "the ramp is a param, not beside them"
        );
        let mut value = g.node(c).expect("c").params["ramp"].clone();
        if lose_cyclic {
            value[1] = 0.0;
        }
        assert_eq!(
            value,
            ramp.to_gradient(),
            "the param is the ramp's gradient"
        );
        assert_eq!(
            Ramp::from_gradient(&value).expect("a ramp"),
            ramp,
            "the gradient reads back as the ramp (cyclic {cyclic})"
        );
        for k in 0..=200 {
            let t = f64::from(k) / 200.0 * 1.5 - 0.25;
            if (t * 4.0).fract().abs() < 1e-9 {
                continue;
            }
            let want = ramp.linear_at(t).expect("finite");
            let got = params::gradient_at(&value, t as f32).expect("a colour");
            for i in 0..3 {
                assert!(
                    (f64::from(got[i]) - want[i]).abs() < 1e-5,
                    "t = {t}: {got:?} vs {want:?} (cyclic {cyclic})"
                );
            }
        }
    }
}

#[test]
fn the_custom_stop_ramp_is_a_gradient_parameter() {
    check_ramp(false);
}

negative_control!(
    the_custom_stop_ramp_is_a_gradient_parameter,
    "a conversion that drops the cyclic flag reads the cyclic ramp differently",
    expected = "the param is the ramp's gradient",
    check_ramp(true)
);

#[test]
fn a_ramp_beside_the_params_still_opens() {
    // Pass criterion: a node whose code declares no gradient keeps the ramp beside its params, as it was saved, and a
    // stain saved with the ramp beside the params of a node that does declare one opens with it as that param.
    let ramp = a_ramp(true);
    let (mut plain, c) =
        graph("fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.inputs[0].x); }");
    plain.set_ramp(c, Some(ramp.clone())).expect("set");
    assert_eq!(
        plain.node(c).expect("c").ramp,
        Some(ramp.clone()),
        "kept beside"
    );
    assert_eq!(plain.ramp_of(c).expect("ramp"), Some(ramp.clone()));
    let back: StainGraph =
        serde_json::from_str(&serde_json::to_string(&plain).expect("saved")).expect("read");
    assert_eq!(
        back, plain,
        "a ramp with no gradient to live in round-trips as it did"
    );

    // The old file: the gradient node's text, with the ramp beside the params and no param.
    let (g, c) = graph(RAMPED);
    let mut old: serde_json::Value = serde_json::to_value(&g).expect("value");
    let node = old["nodes"]
        .as_array_mut()
        .expect("nodes")
        .iter_mut()
        .find(|n| n["id"] == serde_json::json!(c.0))
        .expect("the node");
    node["ramp"] = serde_json::to_value(&ramp).expect("ramp");
    let opened: StainGraph = serde_json::from_value(old).expect("opens");
    let n = opened.node(c).expect("c");
    assert_eq!(n.ramp, None, "the ramp moved into the gradient");
    assert_eq!(n.params["ramp"], ramp.to_gradient());
    assert_eq!(opened.ramp_of(c).expect("ramp"), Some(ramp));
}

negative_control!(
    a_ramp_beside_the_params_still_opens,
    "a ramp that is not the one saved fails the same comparison",
    expected = "ramp moved",
    {
        let (g, c) = graph(RAMPED);
        let mut old: serde_json::Value = serde_json::to_value(&g).expect("value");
        let node = old["nodes"]
            .as_array_mut()
            .expect("nodes")
            .iter_mut()
            .find(|n| n["id"] == serde_json::json!(c.0))
            .expect("the node");
        node["ramp"] = serde_json::to_value(a_ramp(false)).expect("ramp");
        let opened: StainGraph = serde_json::from_value(old).expect("opens");
        assert_eq!(
            opened.node(c).expect("c").params["ramp"],
            a_ramp(true).to_gradient(),
            "the ramp moved into the gradient"
        );
    }
);

// ── A pair ────────────────────────────────────────────────────────────────────────────────────────────────────────

const PAIRED: &str = "// @version 2
// @uniform lo: f32 = 0.25 [0, 10]
// @uniform hi: f32 = 4.0 [0, 10]
// @control lo slider \"Low\" pair hi soft [0, 5] unit \"m\" format fixed(2)
// @control hi slider \"High\" unit \"m\"
fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.inputs[0].x * uniforms.lo + uniforms.hi); }
";

fn check_pair(plant: bool) {
    // Pass criterion: a low end above the high end's value, set or default, is refused; at or below it, accepted.
    let (mut g, c) = graph(PAIRED);
    g.set_param(c, "lo", vec![4.0]).expect("equal ends");
    g.set_param(c, "lo", vec![1.0]).expect("below");
    assert!(
        g.set_param(c, "lo", vec![4.5]).is_err(),
        "a low end above its high end was accepted"
    );
    g.set_param(c, "hi", vec![6.0]).expect("raised");
    g.set_param(c, "lo", vec![5.0]).expect("below the new high");
    assert!(
        g.set_param(c, "hi", vec![if plant { 5.0 } else { 4.0 }])
            .is_err(),
        "a high end below its low end was accepted"
    );
}

#[test]
fn a_pair_stays_ordered() {
    check_pair(false);
}

negative_control!(
    a_pair_stays_ordered,
    "a high end set equal to the low end is not below it, so it is accepted",
    expected = "a high end below its low end was accepted",
    check_pair(true)
);

// ── A field read in the code ──────────────────────────────────────────────────────────────────────────────────────

const FIELDED: &str = "// @version 2
// @uniform src: field = d_min
// @uniform gain: f32 = 0.5 [0, 1]
fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(uniforms.src * uniforms.gain); }
";

fn check_field(second: &str) {
    // Pass criterion: picking another field is another code, so another fragment key, and the picked field is read, so
    // it is in the assembled stain's fields; setting the slider is the same code and the same key.
    let (mut g, c) = graph(FIELDED);
    let key = |g: &StainGraph| g.canonical().expect("form").fragment_key();
    let first = key(&g);
    g.set_param(c, "gain", vec![0.25]).expect("gain");
    assert_eq!(key(&g), first, "a slider is no code");
    g.set_param(c, "src", vec![field_index(second)])
        .expect("a field");
    assert_ne!(key(&g), first, "a different field is a different code");
    let f = assemble::assemble(g.canonical().expect("form").stain(), assemble::Tier::FULL)
        .expect("assembles");
    assert!(
        f.fields.iter().any(|x| x == second),
        "the picked field is read: {:?}",
        f.fields
    );
}

#[test]
fn a_field_parameter_is_read_in_the_code() {
    check_field("ftle");
}

negative_control!(
    a_field_parameter_is_read_in_the_code,
    "a pick of the field it already was is the same code",
    expected = "a different field is a different code",
    check_field("d_min")
);

// ── A wire an input does not take ─────────────────────────────────────────────────────────────────────────────────

fn noting(accepts: &str) -> String {
    format!(
        "// @version 2\n// @input depth\n// @port depth \"Depth\" {accepts} optional unit \"m\"\n\
         fn colour(ctx: Ctx) -> vec3<f32> {{ return vec3<f32>(ctx.inputs[0].x); }}\n"
    )
}

fn check_note(accepts: &str) {
    // Pass criterion: a categorical field wired into an input that takes scalars attaches, and is noted once, with the
    // input's label; wiring a scalar into it is not noted.
    let mut g = StainGraph::new();
    let cat = g
        .add(
            NodeKind::Source,
            Occupant::Field(field_of(FieldSubtype::Categorical)),
        )
        .expect("a source");
    let sc = g
        .add(
            NodeKind::Source,
            Occupant::Field(field_of(FieldSubtype::Scalar)),
        )
        .expect("a source");
    let c = g
        .add(NodeKind::Colour, Occupant::Custom(noting(accepts)))
        .expect("a colour");
    g.connect(sc, c, 0).expect("scalar");
    assert!(
        g.wire_notes().expect("notes").is_empty(),
        "a scalar is taken"
    );
    g.connect(cat, c, 0)
        .expect("a wire it does not take still attaches");
    let notes = g.wire_notes().expect("notes");
    assert_eq!(notes.len(), 1, "a note is given: {notes:?}");
    assert_eq!(notes[0].label, "Depth");
}

#[test]
fn a_wire_an_input_does_not_take_is_noted_and_still_attaches() {
    check_note("accepts scalar");
}

negative_control!(
    a_wire_an_input_does_not_take_is_noted_and_still_attaches,
    "an input that declares no subtypes takes any, so nothing is noted",
    expected = "a note is given",
    check_note("")
);

// ── The description ───────────────────────────────────────────────────────────────────────────────────────────────

fn check_description(port_unit: &str) {
    // Pass criterion: the pair, soft range, unit and format are on the low end's view, the gradient's capacity and the
    // port's label, subtypes and unit are as declared, and an input with no `@port` is its name, taking anything.
    let text = format!(
        "{}// @input extra\n// @port depth \"Depth\" accepts scalar | position optional unit \"{port_unit}\"\n",
        PAIRED.replace(
            "fn colour",
            "// @input depth\n// @uniform g: gradient<6> = linear 0:(0,0,0) 1:(1,1,1)\n\
             // @control g gradient \"G\"\n// @glyph gradient\nfn colour"
        )
    );
    let d = describe(
        NodeKind::Colour,
        &Occupant::Custom(text),
        &Default::default(),
    )
    .expect("described");
    let lo = d.controls.iter().find(|c| c.uniform == "lo").expect("lo");
    assert_eq!(lo.pair.as_deref(), Some("hi"));
    assert_eq!(lo.soft, Some((0.0, 5.0)));
    assert_eq!(lo.unit.as_deref(), Some("m"));
    assert_eq!(
        lo.format,
        Some(engine::contract::stain::describe::Format::Fixed(2))
    );
    assert_eq!(
        d.glyph,
        engine::contract::stain::describe::Shown::Gradient(vec![[0.0; 3], [1.0; 3]]),
        "the gradient glyph draws the gradient's own stops"
    );
    let g = d.controls.iter().find(|c| c.uniform == "g").expect("g");
    assert_eq!(g.capacity, Some(6));
    assert_eq!(g.value.len(), 10);
    let depth = d.ports.iter().find(|p| p.input == "depth").expect("depth");
    assert_eq!(depth.label, "Depth");
    assert!(depth.optional);
    assert_eq!(depth.unit.as_deref(), Some("m"), "the port's unit");
    assert_eq!(depth.accepts.len(), 2);
    let extra = d.ports.iter().find(|p| p.input == "extra").expect("extra");
    assert_eq!(extra.label, "extra");
    assert!(extra.accepts.is_empty() && !extra.optional);
}

// ── The proof of what matters, through the engine ────────────────────────────────────────────────────────────────

const DEAD_SPARE: &str = "// @version 2
// @uniform ramp: gradient<8> = oklab 0:(0.25, 0.0, 0.0) 1:(0.875, 0.0, 0.0)
// @uniform spare: gradient<4> = linear 0:(0.0, 0.0, 0.0) 1:(1.0, 1.0, 1.0)
fn colour(ctx: Ctx) -> vec3<f32> {
    let unused = gradient4_at(uniforms.spare, 0.5);
    return gradient8_at(uniforms.ramp, ctx.inputs[0].x);
}
";

fn check_standing(read_spare: bool) {
    // Pass criterion: the editor's standing of `ramp` is that it matters and of `spare`, read into nothing, that it
    // never can; with `spare` read into the result, it matters.
    let text = if read_spare {
        DEAD_SPARE.replace(
            "return gradient8_at(uniforms.ramp, ctx.inputs[0].x);",
            "return gradient8_at(uniforms.ramp, ctx.inputs[0].x) + unused;",
        )
    } else {
        DEAD_SPARE.to_owned()
    };
    let s = standing(
        NodeKind::Colour,
        &Occupant::Custom(text),
        &Default::default(),
    )
    .expect("standing");
    assert_eq!(s[0].matters, Matters::Now, "the ramp matters");
    assert_eq!(
        s[1].matters,
        Matters::Never,
        "the spare gradient never matters"
    );
}

#[test]
fn a_dead_gradient_is_never_mattering() {
    check_standing(false);
}

negative_control!(
    a_dead_gradient_is_never_mattering,
    "a gradient read into the result matters",
    expected = "the spare gradient never matters",
    check_standing(true)
);

#[test]
fn the_description_carries_what_a_source_declares() {
    check_description("m");
}

negative_control!(
    the_description_carries_what_a_source_declares,
    "a port declared in seconds is not the metres the check expects",
    expected = "the port's unit",
    check_description("s")
);

// ── A field parameter before a slider ─────────────────────────────────────────────────────────────────────────────

fn field_then_slider(reads_gain: bool) -> String {
    format!(
        "// @version 2\n// @uniform src: field = d_min\n// @uniform gain: f32 = 0.5 [0, 1]\n\
         // @control gain slider \"Gain\"\n\
         fn colour(ctx: Ctx) -> vec3<f32> {{ return vec3<f32>(uniforms.src{}); }}\n",
        if reads_gain { " * uniforms.gain" } else { "" }
    )
}

fn check_field_standing(reads_gain: bool) {
    // Pass criterion: the editor's standing of a node whose field is declared before its slider is no error, and says
    // the slider matters where the code reads it.
    let s = standing(
        NodeKind::Colour,
        &Occupant::Custom(field_then_slider(reads_gain)),
        &Default::default(),
    )
    .expect("standing is no error");
    assert_eq!(
        s[1].matters,
        Matters::Now,
        "the slider after a field matters"
    );
}

#[test]
fn a_field_parameter_does_not_break_the_standing_of_the_controls_after_it() {
    check_field_standing(true);
}

negative_control!(
    a_field_parameter_does_not_break_the_standing_of_the_controls_after_it,
    "a slider the code never reads does not matter",
    expected = "the slider after a field matters",
    check_field_standing(false)
);

// ── A field is saved by its name ──────────────────────────────────────────────────────────────────────────────────

fn saved_field(shift: bool) -> serde_json::Value {
    // Pass criterion: a saved stain names its field, and the ledger's order is nowhere in it: the field it opens as is
    // the one saved, though the list's indices move. `shift` writes the old, index form and moves the index by one, as
    // a reordered ledger would.
    let (mut g, c) = graph(FIELDED);
    g.set_param(c, "src", vec![field_index("ftle")])
        .expect("a field");
    let mut saved: serde_json::Value = serde_json::to_value(&g).expect("saved");
    let node = saved["nodes"]
        .as_array_mut()
        .expect("nodes")
        .iter_mut()
        .find(|n| n["id"] == serde_json::json!(c.0))
        .expect("the node")
        .clone();
    assert_eq!(node["fields"]["src"], "ftle", "the field is saved by name");
    assert!(node["params"].get("src").is_none(), "no index is saved");
    let mut opened = saved.clone();
    if shift {
        let n = opened["nodes"]
            .as_array_mut()
            .expect("nodes")
            .iter_mut()
            .find(|n| n["id"] == serde_json::json!(c.0))
            .expect("the node");
        n.as_object_mut().expect("node").remove("fields");
        n["params"]["src"] = serde_json::json!([field_index("ftle") + 1.0]);
    }
    let back: StainGraph = serde_json::from_value(opened).expect("opens");
    let names = assemble::source_fields().expect("fields");
    let at = back.node(c).expect("c").params["src"][0] as usize;
    assert_eq!(names[at], "ftle", "the saved stain names the same field");
    saved
}

#[test]
fn a_field_is_saved_by_name() {
    saved_field(false);
}

negative_control!(
    a_field_is_saved_by_name,
    "a field saved by index names another field once the order moves",
    expected = "the saved stain names the same field",
    {
        saved_field(true);
    }
);

#[test]
fn a_field_that_is_gone_opens_with_the_node_saying_so() {
    // Pass criterion: a stain naming a field this build has none of opens; the node holds the name it lost and the
    // uniform at its default; saved again, it names the same field.
    let (g, c) = graph(FIELDED);
    let mut saved: serde_json::Value = serde_json::to_value(&g).expect("saved");
    let node = saved["nodes"]
        .as_array_mut()
        .expect("nodes")
        .iter_mut()
        .find(|n| n["id"] == serde_json::json!(c.0))
        .expect("the node");
    node["fields"] = serde_json::json!({ "src": "a_field_since_removed" });
    let opened: StainGraph = serde_json::from_value(saved).expect("opens");
    let n = opened.node(c).expect("c");
    assert_eq!(n.lost_fields["src"], "a_field_since_removed");
    assert!(
        !n.params.contains_key("src"),
        "the uniform is at its default"
    );
    let again = serde_json::to_value(&opened).expect("saved again");
    let kept = again["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .find(|n| n["id"] == serde_json::json!(c.0))
        .expect("the node");
    assert_eq!(
        kept["fields"]["src"], "a_field_since_removed",
        "still named"
    );
}

negative_control!(
    a_field_that_is_gone_opens_with_the_node_saying_so,
    "a stain whose field is present has nothing lost",
    expected = "a field is lost",
    {
        let (g, c) = graph(FIELDED);
        let saved = serde_json::to_value(&g).expect("saved");
        let opened: StainGraph = serde_json::from_value(saved).expect("opens");
        assert!(
            !opened.node(c).expect("c").lost_fields.is_empty(),
            "a field is lost"
        );
    }
);

// ── A value is no code ────────────────────────────────────────────────────────────────────────────────────────────

fn check_value_is_no_code(key: fn(&StainGraph) -> u64) {
    // Pass criterion: changing a gradient's, a list's or a lut's value changes no key of the code.
    let (mut g, c) = graph(EVERY_TYPE);
    let before = key(&g);
    for (name, v) in values().into_iter().take(3) {
        g.set_param(c, name, v).expect("a value");
    }
    assert_eq!(key(&g), before, "a value is no code");
}

#[test]
fn a_gradient_list_or_lut_value_leaves_the_fragment_key_alone() {
    check_value_is_no_code(|g| g.canonical().expect("form").fragment_key());
}

negative_control!(
    a_gradient_list_or_lut_value_leaves_the_fragment_key_alone,
    "the render key hashes the values in, so it changes with them",
    expected = "a value is no code",
    check_value_is_no_code(|g| g.canonical().expect("form").render_key())
);
