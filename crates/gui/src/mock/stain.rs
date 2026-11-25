//! The mock's stain (R-390, ORDER item 6): the graph it starts with, the look its preview is drawn in, and the Stain
//! editor's content it serves. The library, the choices, each node's description and its code are the registry's; the
//! assembled code and the Problems pane's lines are placeholders, the mock assembling nothing.
//! The graph is a real stain graph, checked as every edit of it is (gui_state_contract §5; `engine::stain`), made of
//! 02_stain.png's nodes: the outcome state and `t_end` as sources, the outcome palette as colour, a late fade as
//! brightness, multiplied at the combiner into `OUT`.

use std::sync::Mutex;

use engine::contract::render_state::{NodeId, NodeKind, Occupant, StainGraph};

use crate::stain::catalogue::catalogue;
use crate::stain::{described, node_code, Content};

/// The source of the outcome class: the payload's `state` (02_stain.png's "Outcome state").
pub const OUTCOME_FIELD: &str = "state";
/// The source of the time of fate: the payload's `t_end_step` (02_stain.png's "Time of fate", `ctx.t_end`).
pub const FATE_FIELD: &str = "t_end_step";
/// The late fade's param: how dark the latest fates go, as a fraction of full.
pub const FADE: &str = "fade";
/// The late fade: full brightness for an early fate, fading to `fade` at the horizon.
pub const LATE_FADE: &str = "// @name Late fade
// @group Brightness
// @about Full brightness for an early fate, fading towards the horizon.
// @maps t_end [lin, 0 to 50]
// @glyph ramp
// @input t_end [0, 50]
// @uniform fade: f32 = 0.12 [0, 1]
// @control fade slider \"Fade\"
fn brightness(ctx: Ctx) -> f32 {
    return mix(1.0, fade, clamp(ctx.inputs[0].x / 50.0, 0.0, 1.0));
}
";

/// The fields the mock's picture tells apart ([`look`]).
pub const SOURCE_FIELDS: [&str; 5] = ["state", "t_end_step", "ftle", "d_min", "energy_drift"];

/// The combiner and `OUT`: the backbone's ids.
pub const COMBINER: NodeId = NodeId(0);
/// See [`COMBINER`].
pub const OUT: NodeId = NodeId(1);

/// The mock's graph: 02_stain.png's six nodes, wired as it draws them.
pub fn default_graph() -> StainGraph {
    let mut g = StainGraph::new();
    let checked = "the mock's graph is a stain graph";
    g.set_occupant(COMBINER, Occupant::Builtin("multiply".into()))
        .expect(checked);
    let outcome = g
        .add(NodeKind::Source, Occupant::Field(OUTCOME_FIELD.into()))
        .expect(checked);
    let fate = g
        .add(NodeKind::Source, Occupant::Field(FATE_FIELD.into()))
        .expect(checked);
    let palette = g
        .add(
            NodeKind::Colour,
            Occupant::Builtin(engine::contract::legend::outcome_palette().into()),
        )
        .expect(checked);
    let fade = g
        .add(NodeKind::Brightness, Occupant::Custom(LATE_FADE.into()))
        .expect(checked);
    for (from, to, port) in [
        (outcome, palette, 0),
        (fate, fade, 0),
        (palette, COMBINER, 0),
        (fade, COMBINER, 1),
    ] {
        g.connect(from, to, port).expect(checked);
    }
    g
}

/// The look the preview is drawn in, from the graph (placeholder content): `[colour, darkest, combiner, phase]`.
/// `colour` is 1 while a colour node feeds the combiner, its field wired, else 0 (a grey ramp); `darkest` the live
/// brightness node's `fade`, 1 with none; `combiner` 0 for Multiply, 1 for Replace-L, 2 for anything else; `phase` the
/// colour's source field's place in [`SOURCE_FIELDS`] over their count, so a new field shifts the picture.
pub type Look = [f32; 4];

/// The look of `g` ([`Look`]).
pub fn look(g: &StainGraph) -> Look {
    let wires = g.wires();
    let feeding = |to: NodeId, port: u32| {
        wires
            .iter()
            .find(|w| w.to == to && w.port == port)
            .map(|w| w.from)
    };
    let live = |id: Option<NodeId>| {
        id.and_then(|id| g.node(id))
            .filter(|n| n.occupant != Occupant::None && feeding(n.id, 0).is_some())
    };
    let colour = live(feeding(COMBINER, 0));
    let field = colour
        .and_then(|n| feeding(n.id, 0))
        .and_then(|id| g.node(id))
        .and_then(|n| match &n.occupant {
            Occupant::Field(f) => SOURCE_FIELDS.iter().position(|s| s == f),
            _ => None,
        });
    let darkest = live(feeding(COMBINER, 1)).map_or(1.0, |n| {
        let declared = g
            .uniforms(n.id)
            .ok()
            .and_then(|us| us.into_iter().find(|u| u.name == FADE))
            .map_or(1.0, |u| u.default[0]);
        n.params.get(FADE).map_or(declared, |v| v[0])
    });
    let combiner = match g.node(COMBINER).map(|n| &n.occupant) {
        Some(Occupant::Builtin(id)) if id == "multiply" => 0.0,
        Some(Occupant::Builtin(id)) if id == "replace_l" => 1.0,
        _ => 2.0,
    };
    [
        if colour.is_some() { 1.0 } else { 0.0 },
        darkest as f32,
        combiner,
        field.map_or(0.0, |i| i as f32 / SOURCE_FIELDS.len() as f32),
    ]
}

/// The occupant as the placeholder code reads it.
fn occupant_name(o: &Occupant) -> &str {
    match o {
        Occupant::None => "none",
        Occupant::Field(f) => f,
        Occupant::Builtin(id) => id,
        Occupant::Custom(_) => "custom",
        Occupant::Pinned(p) => &p.id,
    }
}

/// The field the node `to`'s first in-port reads, through its source, or `none`.
fn field_into(g: &StainGraph, to: Option<NodeId>) -> String {
    to.and_then(|to| g.wires().into_iter().find(|w| w.to == to && w.port == 0))
        .and_then(|w| g.node(w.from))
        .map_or_else(
            || "none".to_owned(),
            |n| occupant_name(&n.occupant).to_owned(),
        )
}

/// The assembled code the Stain page shows under the canvas (placeholder content in 02_stain.png's form): it reads
/// the graph's combiner and the fields its colour and brightness read, so an edit shows in it.
pub fn assembled(g: &StainGraph) -> String {
    let wires = g.wires();
    let into = |port: u32| {
        wires
            .iter()
            .find(|w| w.to == COMBINER && w.port == port)
            .map(|w| w.from)
    };
    let (colour, brightness) = (into(0), into(1));
    let combine = match g.node(COMBINER).map(|n| occupant_name(&n.occupant)) {
        Some("multiply") => "c * b",
        Some("replace_l") => "with_l(c, b)",
        _ => "c",
    };
    format!(
        "// stain: outcome class — generated from the graph · node bounds\n\
         fn colour(cls: u32) -> vec3<f32> {{ return PAL[cls]; }}\n\
         fn brightness(t: f32) -> f32 {{ return curve(range_norm(t, 0.0, 50.0)); }}\n\
         fn combine(c: vec3<f32>, b: f32) -> vec3<f32> {{ return {combine}; }}\n\
         fn shade(ctx: SampleCtx) -> vec3<f32> {{\n  \
         return combine(colour(ctx.{}), brightness(ctx.{}));\n}}\n",
        field_into(g, colour),
        field_into(g, brightness),
    )
}

/// The whole pipeline's code, the Pipeline WGSL view (placeholder content): the assembled code after the prelude's
/// line and each node's function, by id.
pub fn pipeline(g: &StainGraph) -> String {
    let mut text =
        String::from("// pipeline WGSL (mock): the prelude, each node's function, then shade()\n");
    text.push_str("#import prelude::{Ctx, SampleCtx, PAL, curve, range_norm, with_l}\n\n");
    for n in g.nodes() {
        text.push_str(&format!(
            "// node {} · {:?} · {}\n",
            n.id.0,
            n.kind,
            occupant_name(&n.occupant)
        ));
    }
    text.push('\n');
    text.push_str(&assembled(g));
    text
}

/// The compile status line (placeholder content, 02_stain.png's).
pub const COMPILED: &str = "✔ compiled · 3.1 ms · pipeline cached";

/// The Problems pane's note (placeholder content, 02_stain.png's).
pub const NOTE: &str = "note: brightness range fixed [0, 50]";

/// What the mock logs when asked to recompile.
pub const RECOMPILED: &str = "stain recompiled · 3.1 ms (mock)";

/// The editor's content for `g`, made once a graph.
pub fn content(g: &StainGraph) -> Content {
    static LAST: Mutex<Option<(StainGraph, Content)>> = Mutex::new(None);
    let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((_, content)) = last.as_ref().filter(|(of, _)| of == g) {
        return content.clone();
    }
    let content = made(g);
    *last = Some((g.clone(), content.clone()));
    content
}

fn made(g: &StainGraph) -> Content {
    let tables = catalogue();
    let (described, undescribed) = described(g);
    let mut problems = tables.problems.clone();
    problems.extend(undescribed);
    Content {
        library: &tables.groups,
        modes: tables.modes,
        loaded: tables.loaded(g),
        code: assembled(g),
        pipeline: pipeline(g),
        described,
        node_code: node_code(g),
        compiled: COMPILED.to_owned(),
        compiling: false,
        problems,
        notes: vec![NOTE.to_owned()],
        choices: &tables.choices,
        graphs: &tables.graphs,
    }
}
