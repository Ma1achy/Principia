//! One description for a built-in and a custom (`engine::contract::stain::describe`): each built-in's source pasted as
//! a custom is described the same in every part; the description's code names no occupant; the worked
//! example is described control by control; and a label edit keeps the fragment and render keys.

use std::collections::BTreeMap;

use engine::contract::stain::assembled::library;
use engine::contract::stain::describe::{describe, Control, Description, Shown};
use engine::contract::stain::graph::{NodeId, NodeKind, Occupant, StainGraph};
use render::assemble;
use render::registry;
use validation::negative_control;

const EXAMPLE: &str = include_str!("../../render/tests/data/every_control.wgsl");

type Params = BTreeMap<String, Vec<f64>>;
type Describer = dyn Fn(NodeKind, &Occupant, &Params) -> Description;

/// Every built-in occupant: each library entry's built-in, and each source's field.
fn builtins() -> Vec<(NodeKind, Occupant)> {
    let mut out: Vec<(NodeKind, Occupant)> = library()
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .map(|e| (e.slot, Occupant::Builtin(e.name)))
        .collect();
    for f in assemble::source_fields().unwrap_or_else(|e| panic!("{e}")) {
        out.push((NodeKind::Source, Occupant::Field(f)));
    }
    out
}

/// Params setting each ranged scalar control to its range's top, so current values are compared, not defaults only.
fn edited(d: &Description) -> Params {
    d.controls
        .iter()
        .filter_map(|c| Some((c.uniform.clone(), vec![c.range?.1])))
        .collect()
}

/// Each built-in, with its params as given and edited, described as `describer` describes it, equals its source pasted
/// as a custom in every part: the description carries nothing that says built-in or custom.
fn check_pasted(describer: &Describer) {
    let all = builtins();
    let count = all.len();
    assert!(count > 50, "only {count} built-ins");
    let mut with_controls = 0;
    for (kind, occupant) in all {
        let text = assemble::occupant_source(kind.into(), &(&occupant).into())
            .unwrap_or_else(|e| panic!("{e}"))
            .unwrap_or_default();
        let pasted = Occupant::Custom(text);
        let plain = describer(kind, &occupant, &Params::new());
        let params = edited(&plain);
        with_controls += usize::from(!params.is_empty());
        for params in [Params::new(), params] {
            let ours = describer(kind, &occupant, &params);
            let theirs = describer(kind, &pasted, &params);
            assert_eq!(
                ours, theirs,
                "{occupant:?}: the built-in and its source pasted as a custom are described differently"
            );
        }
    }
    eprintln!("{count} built-ins compared, {with_controls} of them also with edited params");
    assert!(
        with_controls >= 2,
        "only {with_controls} built-ins have a ranged control"
    );
}

fn plain(kind: NodeKind, occupant: &Occupant, params: &Params) -> Description {
    describe(kind, occupant, params).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn description_is_one_for_builtin_and_custom() {
    check_pasted(&plain);
}

negative_control!(
    description_is_one_for_builtin_and_custom,
    "a description that names a built-in by its id",
    expected = "are described differently",
    check_pasted(&|kind, occupant, params| {
        let mut d = plain(kind, occupant, params);
        if let Occupant::Builtin(id) = occupant {
            d.name = id.clone();
        }
        d
    })
);

// ── No occupant in the description's code ─────────────────────────────────────────────────────────────────────────

/// The description's code: the engine's describe and the render side's grammar.
const CODE: [(&str, &str); 2] = [
    (
        "engine/src/contract/stain/describe.rs",
        include_str!("../src/contract/stain/describe.rs"),
    ),
    (
        "render/src/assemble/declare.rs",
        include_str!("../../render/src/assemble/declare.rs"),
    ),
];

/// The string literals of `text`'s code, its comments dropped.
fn literals(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines().filter(|l| !l.trim_start().starts_with("//")) {
        let mut rest = line;
        while let Some(start) = rest.find('"') {
            let body = &rest[start + 1..];
            let mut end = None;
            let mut escaped = false;
            for (k, c) in body.char_indices() {
                match c {
                    '\\' if !escaped => escaped = true,
                    '"' if !escaped => {
                        end = Some(k);
                        break;
                    }
                    _ => escaped = false,
                }
            }
            let Some(end) = end else { break };
            out.push(body[..end].to_owned());
            rest = &body[end + 1..];
        }
    }
    out
}

/// Every occupant's id: each registry entry's whole id and its name, and each source's field.
fn ids() -> Vec<String> {
    let mut out = Vec::new();
    for e in registry::registry().unwrap_or_else(|e| panic!("{e}")) {
        out.push(e.id.rsplit('/').next().unwrap_or(&e.id).to_owned());
        out.push(e.id);
    }
    out.extend(assemble::source_fields().unwrap_or_else(|e| panic!("{e}")));
    out
}

/// No file of `code` holds an occupant's id as a string, or names the built-in occupant variant or a built-in's module.
fn check_no_ids(code: &[(&str, String)]) {
    let ids = ids();
    for (file, text) in code {
        for lit in literals(text) {
            assert!(
                !ids.contains(&lit),
                "{file} names the occupant `{lit}`: the description must come from the source alone"
            );
        }
        for word in ["Builtin", "BuiltIn", "outcome::"] {
            let code: String = text
                .lines()
                .filter(|l| !l.trim_start().starts_with("//"))
                .collect();
            assert!(
                !code.contains(word),
                "{file} names the occupant `{word}`: the description must come from the source alone"
            );
        }
    }
}

fn code() -> Vec<(&'static str, String)> {
    CODE.iter().map(|(f, t)| (*f, (*t).to_owned())).collect()
}

#[test]
fn description_code_names_no_occupant() {
    check_no_ids(&code());
}

negative_control!(
    description_code_names_no_occupant,
    "a planted id",
    expected = "names the occupant",
    check_no_ids(&{
        let mut c = code();
        c[0].1
            .push_str("\nconst PLANTED: &str = \"outcome_state\";\n");
        c
    })
);

// ── The worked example ─────────────────────────────────────────────────────────────────────────────────────────────

fn check_example(text: &str) {
    let params: Params = [("strength".to_owned(), vec![0.75])].into();
    let d = plain(
        NodeKind::Colour,
        &Occupant::Custom(text.to_owned()),
        &params,
    );
    assert_eq!(d.name, "Banded tint");
    assert_eq!(d.group, "Examples");
    assert_eq!(
        d.about,
        "A field's value as bands of a gradient, tinted, with every control the editor draws."
    );
    assert_eq!(d.maps, "maps its field [lin, 0 to 1] → vec3");
    let order: Vec<(&str, &str, &str)> = d
        .controls
        .iter()
        .map(|c| (c.uniform.as_str(), c.label.as_str(), c.control.word()))
        .collect();
    assert_eq!(
        order,
        [
            ("mode", "Mode", "choice"),
            ("bands", "Bands", "stepper"),
            ("scale", "Scale", "log_slider"),
            ("strength", "Strength", "slider"),
            ("invert", "Invert", "toggle"),
            ("mask", "Channels", "flags"),
            ("warm", "Warm", "colour"),
            ("cool", "Cool", "colour"),
            ("hue", "Hue turn", "angle"),
            ("low", "Ramp", "gradient"),
            ("offset", "Offset", "fields"),
            ("measured", "Measured range", "hidden"),
        ],
        "the controls are not the example's"
    );
    let strength = &d.controls[3];
    assert_eq!(
        (
            strength.value.as_slice(),
            strength.default.as_slice(),
            strength.range,
            strength.group.as_deref()
        ),
        (
            [0.75].as_slice(),
            [0.5].as_slice(),
            Some((0.0, 1.0)),
            Some("Shape")
        )
    );
    assert_eq!(
        d.controls[0].control,
        Control::Choice(vec!["bands".into(), "smooth".into(), "steps".into()])
    );
    let stops: Vec<&str> = d.controls[9]
        .stops
        .iter()
        .map(|s| s.uniform.as_str())
        .collect();
    assert_eq!(stops, ["mid", "high"]);
    match &d.glyph {
        Shown::Swatches(chips) => assert_eq!(
            chips
                .iter()
                .map(|c| (c.uniform.as_str(), c.rgb))
                .collect::<Vec<_>>(),
            [("warm", [1.0, 0.5, 0.2]), ("cool", [0.2, 0.4, 1.0])]
        ),
        other => panic!("the glyph is {other:?}"),
    }
    let gradient = text.replace("// @glyph swatches \"Tints\"", "// @glyph gradient");
    let d = plain(NodeKind::Colour, &Occupant::Custom(gradient), &params);
    assert_eq!(d.glyph, Shown::Gradient(vec![[0.0; 3], [0.5; 3], [1.0; 3]]));
}

#[test]
fn description_of_the_worked_example() {
    check_example(EXAMPLE);
}

negative_control!(
    description_of_the_worked_example,
    "the example's order lines dropped",
    expected = "the controls are not the example's",
    check_example(&EXAMPLE.replace(" order 0", ""))
);

/// A swatches glyph whose group a choice names draws the colours of the group the choice has chosen, their current
/// values; a choice of no colours leaves it the glyph's own.
fn check_chosen_swatches(describer: &Describer) {
    let text = "// @glyph swatches \"A\"\n\
                // @uniform set: u32 = 0 [0, 2]\n// @control set choice(\"A\", \"B\", \"none\") \"Set\"\n\
                // @uniform a: vec3<f32> = (1.0, 0.0, 0.0)\n// @control a colour \"a\" in \"A\"\n\
                // @uniform b: vec3<f32> = (0.0, 1.0, 0.0)\n// @control b colour \"b\" in \"B\"\n\
                fn colour(ctx: Ctx) -> vec3<f32> { return select(uniforms.a, uniforms.b, uniforms.set == 1u); }\n";
    for (set, uniform, rgb) in [
        (0.0, "a", [1.0, 0.0, 0.0]),
        (1.0, "b", [0.0, 0.0, 1.0]),
        (2.0, "a", [1.0, 0.0, 0.0]),
    ] {
        let params: Params = [
            ("set".to_owned(), vec![set]),
            ("b".to_owned(), vec![0.0, 0.0, 1.0]),
        ]
        .into();
        let d = describer(NodeKind::Colour, &Occupant::Custom(text.into()), &params);
        let Shown::Swatches(chips) = d.glyph else {
            panic!("the glyph is {:?}", d.glyph);
        };
        assert_eq!(
            chips
                .iter()
                .map(|c| (c.uniform.as_str(), c.rgb))
                .collect::<Vec<_>>(),
            [(uniform, rgb)],
            "the chips are not the chosen set's"
        );
    }
}

#[test]
fn description_swatches_follow_the_set_chosen() {
    check_chosen_swatches(&plain);
}

negative_control!(
    description_swatches_follow_the_set_chosen,
    "a describer blind to the set chosen",
    expected = "the chips are not the chosen set's",
    check_chosen_swatches(&|kind, occupant, params| {
        let mut params = params.clone();
        params.remove("set");
        plain(kind, occupant, &params)
    })
);

/// A source with nothing declared takes its kind's defaults: its first function's name, "Custom", its kind's glyph and
/// maps line, a slider for a ranged scalar labelled by its uniform.
fn check_defaults(describer: &Describer) {
    let text = "// @uniform gain: f32 = 1.0 [0.0, 4.0]\nfn tone(x: f32) -> f32 { return x * uniforms.gain; }\n\
                fn brightness(ctx: Ctx) -> f32 { return tone(ctx.inputs[0].x); }\n";
    let d = describer(
        NodeKind::Brightness,
        &Occupant::Custom(text.to_owned()),
        &Params::new(),
    );
    assert_eq!(
        (d.name.as_str(), d.group.as_str(), d.maps.as_str(), &d.glyph),
        ("tone", "Custom", "maps its field → f32", &Shown::Ramp),
        "the defaults are not the kind's"
    );
    assert_eq!(
        (d.controls[0].label.as_str(), &d.controls[0].control),
        ("gain", &Control::Slider)
    );
    let bare = "fn brightness(ctx: Ctx) -> f32 { return ctx.inputs[0].x; }\n";
    let d = describer(
        NodeKind::Brightness,
        &Occupant::Custom(bare.into()),
        &Params::new(),
    );
    assert_eq!(d.name, "Custom");
}

#[test]
fn description_defaults_come_from_the_kind() {
    check_defaults(&plain);
}

negative_control!(
    description_defaults_come_from_the_kind,
    "a describer that groups every custom under its kind",
    expected = "the defaults are not the kind's",
    check_defaults(&|kind, occupant, params| {
        let mut d = plain(kind, occupant, params);
        d.group = render::assemble::Kind::from(kind).name().to_owned();
        d
    })
);

// ── A label edit keeps the keys ───────────────────────────────────────────────────────────────────────────────────

/// A field into the example's colour, into the pass-through combiner.
fn graph(text: &str) -> StainGraph {
    let mut g = StainGraph::new();
    let s = g
        .add(NodeKind::Source, Occupant::Field("d_min".into()))
        .unwrap_or_else(|e| panic!("{e}"));
    let c = g
        .add(NodeKind::Colour, Occupant::Custom(text.to_owned()))
        .unwrap_or_else(|e| panic!("{e}"));
    g.connect(s, c, 0).unwrap_or_else(|e| panic!("{e}"));
    g.connect(c, NodeId(0), 0).unwrap_or_else(|e| panic!("{e}"));
    g
}

fn keys(text: &str) -> (u64, u64) {
    let c = graph(text).canonical().unwrap_or_else(|e| panic!("{e}"));
    (c.fragment_key(), c.render_key())
}

fn check_label_edit(edited: &str) {
    assert_eq!(
        keys(EXAMPLE),
        keys(edited),
        "the edit changed the fragment or render key"
    );
}

#[test]
fn description_label_edit_keeps_the_keys() {
    check_label_edit(&EXAMPLE.replace("\"Strength\"", "\"Amount\""));
    check_label_edit(&EXAMPLE.replace("// @name Banded tint", "// @name Tinted bands"));
}

negative_control!(
    description_label_edit_keeps_the_keys,
    "a default's edit is code",
    expected = "the edit changed the fragment or render key",
    check_label_edit(&EXAMPLE.replace("strength: f32 = 0.5", "strength: f32 = 0.25"))
);
