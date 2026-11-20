//! qa's tests for TASK-M7-12 through the editor's graph (REQ-GUI-027, REQ-RENDER-072, REQ-COL-025, REQ-GUI-024;
//! R-814: the crate that holds `StainGraph`), written from the requirements and their sources, render_gui_spec
//! Part II §4, §5, §7 and §13 and colour_composition §4.1 and §4.2, not from the implementation. The assembler's own
//! tests are `crates/render/tests/qa_TASK-M7-12.rs`.
//!
//! Every graph here is built and changed by the typed edits alone (`StainEdit` through `apply`), lowered, rendered
//! through the assembler's total entry point and drawn on the GPU:
//! - REQ-GUI-027: the colour node of a two-slot graph deleted, the draw is the greyscale of the brightness, under
//!   Multiply (`white · B`) and under Replace-L (`OKLab(L = B, 0, 0)`);
//! - REQ-RENDER-072: every one of the 64 sets of free nodes deleted from a full graph, one node at a time, each
//!   state renders as a graph, never the fallback and never an error, and draws. This is
//!   exhaustive over deletions and bounded; the random edit sequences are the implementer's property;
//! - REQ-COL-025: 8 posts are taken into the chain and a 9th is refused; moving a post in the chain, and inserting
//!   one, change the draw to the new order's;
//! - REQ-GUI-024: picking a source's field sets its out-port's subtype, vector for the shape `n`, and a source has no
//!   in-port.
//!
//! Exactness: the sample's values and the occupants' constants are exact in binary and every occupant is a product or
//! a swizzle, so a draw is compared with its mirror bit for bit. The one colour that goes through OKLab, Replace-L's
//! `OKLab(L = B, 0, 0)`, is compared in linear sRGB within REQ-COL-049's round-trip tolerance, read from
//! `fixtures/gates/oklab-roundtrip/gate.json` (provisional, R-182): an achromatic OKLab `(L, 0, 0)` is linear `L³` on
//! each channel, by Ottosson's matrices, whose rows from LMS sum to 1.
//!
//! Every check takes its subject as an argument, and its negative control runs it on a wrong one (R-176).

use std::collections::BTreeMap;
use std::path::Path;

use engine::contract::set_field::{Edit, RenderField};
use engine::contract::stain::edit::{apply, Refusal, StainEdit};
use engine::contract::stain::ports::{PortType, Subtype};
use engine::stain::{NodeId, NodeKind, Occupant, StainGraph};
use ledger::gen::prelude;
use render::assemble::{self, AssembleError, Fragment, Rendered, Tier, View};
use render::pipeline_cache::{block_layout, encode};
use validation::gpu::{BindingKind, GpuHarness};
use validation::negative_control;

// ── The draw ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// The sample the entry fills: `n` and `ftle`.
const N: [f32; 3] = [0.5, 0.25, 0.125];
const FTLE: f32 = 0.75;

const ENTRY: &str = r"
@fragment
fn qa_shade(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<u32> {
    var ctx: Ctx;
    ctx.frag_xy = pos.xy;
    ctx.sample.n = vec3<f32>(0.5, 0.25, 0.125);
    ctx.sample.ftle = 0.75;
    return vec4<u32>(bitcast<vec3<u32>>(shade(ctx)), 0u);
}
";

/// `fragment` drawn: `shade()`'s linear RGB at one pixel, each node's uniforms at their defaults; or why not.
fn draw(gpu: &GpuHarness, fragment: &Fragment) -> Result<[f32; 3], String> {
    let source = fragment.source.clone() + ENTRY;
    let mut kinds = vec![BindingKind::Uniform];
    kinds.extend(fragment.uniforms.iter().map(|_| BindingKind::Uniform));
    let kernel = gpu
        .fragment(&source, "qa_shade", &[&kinds], 1, 1)
        .map_err(|e| e.to_string())?;
    let blocks: Vec<Vec<u32>> = fragment
        .uniforms
        .iter()
        .map(|block| {
            let (offsets, size) = block_layout(&block.uniforms);
            let mut bytes = vec![0u8; size as usize];
            for (u, &at) in block.uniforms.iter().zip(&offsets) {
                let value = encode(u.ty, &u.default);
                bytes[at as usize..at as usize + value.len()].copy_from_slice(&value);
            }
            bytes
                .chunks(4)
                .map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]]))
                .collect()
        })
        .collect();
    let prelude_words = prelude::uniform_words(0);
    let mut group: Vec<&[u32]> = vec![&prelude_words];
    group.extend(blocks.iter().map(|w| &w[..]));
    let px = kernel.draw(&[&group]).map_err(|e| e.to_string())?;
    Ok([px[0][0], px[0][1], px[0][2]].map(f32::from_bits))
}

/// What renders a node list: the assembler's total entry point, or a control's stand-in.
type Render = fn(Vec<assemble::Node>) -> Result<Rendered, AssembleError>;

fn total(nodes: Vec<assemble::Node>) -> Result<Rendered, AssembleError> {
    assemble::assemble_or_fallback(nodes, Tier::FULL, View::Presentation)
}

/// `g` lowered and rendered through `render` as a graph: its fragment, never the fallback and never an error.
fn rendered(g: &StainGraph, render: Render) -> Result<Fragment, String> {
    let stain = g
        .lower()
        .map_err(|e| format!("the graph does not lower: {e}"))?;
    let r = render(stain.nodes().to_vec()).map_err(|e| format!("a state did not render: {e}"))?;
    match r.invalid {
        Some(why) => Err(format!("a state rendered the fallback: {why}")),
        None => Ok(r.fragment),
    }
}

/// `g` rendered and drawn.
fn colour_of(gpu: &GpuHarness, g: &StainGraph) -> [f32; 3] {
    let fragment = rendered(g, total).unwrap_or_else(|e| panic!("{e}"));
    draw(gpu, &fragment).unwrap_or_else(|e| panic!("a state did not draw: {e}"))
}

fn bits(c: [f32; 3]) -> [u32; 3] {
    c.map(f32::to_bits)
}

/// REQ-COL-049's round-trip tolerance, read from the gate's configuration, with its requirement checked.
fn tolerance() -> f64 {
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    let config =
        validation::gate::config(root, "oklab-roundtrip").expect("the oklab-roundtrip gate config");
    assert_eq!(config.threshold.requirement, "REQ-COL-049");
    config.threshold.value
}

// ── The edits ────────────────────────────────────────────────────────────────────────────────────────────────────

const COMBINER: NodeId = NodeId(0);
const OUT: NodeId = NodeId(1);

/// `edit` applied to `g` where the editor accepts it; refused, `g` is as it was.
fn edit(g: &mut StainGraph, edit: StainEdit) -> Result<(), Refusal> {
    let set = apply(g, &edit)?;
    match set.edit {
        Edit::Render(RenderField::StainGraph(next)) => *g = next,
        other => panic!("{edit:?} wrote something other than the stain graph: {other:?}"),
    }
    Ok(())
}

/// `edit`, which the editor must accept.
fn must(g: &mut StainGraph, e: StainEdit) {
    if let Err(why) = edit(g, e.clone()) {
        panic!("{e:?} was refused: {why:?}");
    }
}

/// A node of `kind` holding `occupant` added by an edit: its id, the one the graph did not hold before.
fn add(g: &mut StainGraph, kind: NodeKind, occupant: Occupant) -> NodeId {
    let before: Vec<NodeId> = g.nodes().map(|n| n.id).collect();
    must(g, StainEdit::Add { kind, occupant });
    g.nodes()
        .map(|n| n.id)
        .find(|id| !before.contains(id))
        .expect("an accepted add adds a node")
}

fn wire(g: &mut StainGraph, from: NodeId, to: NodeId, port: u32) {
    must(g, StainEdit::Wire { from, to, port });
}

fn custom(text: &str) -> Occupant {
    Occupant::Custom(text.to_owned())
}

fn builtin(id: &str) -> Occupant {
    Occupant::Builtin(id.to_owned())
}

fn field(name: &str) -> Occupant {
    Occupant::Field(name.to_owned())
}

const COLOUR: &str = "fn colour(ctx: Ctx) -> vec3<f32> { return ctx.inputs[0].xyz; }";
const BRIGHTNESS: &str = "fn brightness(ctx: Ctx) -> f32 { return ctx.inputs[0].x; }";
const ROTATE: &str = "fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb.yzx; }";
const QUARTER_G: &str =
    "fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb * vec3<f32>(1.0, 0.25, 1.0); }";
const HALVE_R: &str =
    "fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb * vec3<f32>(0.5, 1.0, 1.0); }";
/// A post with a field in after its colour.
const MASK: &str = "// @input mask
fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb * ctx.inputs[0].x; }";

/// A post's mirror, in f32.
type Mirror = fn([f32; 3]) -> [f32; 3];

fn rotate([r, g, b]: [f32; 3]) -> [f32; 3] {
    [g, b, r]
}

fn quarter_g([r, g, b]: [f32; 3]) -> [f32; 3] {
    [r, g * 0.25, b]
}

fn halve_r([r, g, b]: [f32; 3]) -> [f32; 3] {
    [r * 0.5, g, b]
}

// ── REQ-GUI-027: the colour node deleted, the output is the greyscale of the brightness ──────────────────────────

/// The two-slot graph's free nodes.
#[derive(Clone, Copy, Debug)]
struct TwoSlot {
    colour: NodeId,
    brightness: NodeId,
}

/// A two-slot graph under `combiner`: a source of `n` into a colour, a source of `ftle` into a brightness.
fn two_slot(combiner: &str) -> (StainGraph, TwoSlot) {
    let mut g = StainGraph::new();
    must(
        &mut g,
        StainEdit::Occupant {
            node: COMBINER,
            occupant: builtin(combiner),
        },
    );
    let n = add(&mut g, NodeKind::Source, field("n"));
    let ftle = add(&mut g, NodeKind::Source, field("ftle"));
    let colour = add(&mut g, NodeKind::Colour, custom(COLOUR));
    let brightness = add(&mut g, NodeKind::Brightness, custom(BRIGHTNESS));
    wire(&mut g, n, colour, 0);
    wire(&mut g, ftle, brightness, 0);
    wire(&mut g, colour, COMBINER, 0);
    wire(&mut g, brightness, COMBINER, 1);
    (g, TwoSlot { colour, brightness })
}

/// The node `which` picks deleted from the two-slot graph, under Multiply and under Replace-L: the node and its wires
/// are gone, the graph still renders as a graph, and its draw, which was no grey, is the greyscale of the brightness
/// `B = ftle` (colour_composition §4.1: `white · B`, and `OKLab(L = B, 0, 0)`, linear `B³`).
fn check_delete(which: fn(TwoSlot) -> NodeId) {
    let gpu = GpuHarness::new().expect("a GPU device");
    let tol = tolerance();
    for combiner in ["multiply", "replace_l"] {
        let (mut g, ids) = two_slot(combiner);
        let before = colour_of(&gpu, &g);
        assert!(
            before[0] != before[1],
            "{combiner}: the two-slot graph is already grey: {before:?}"
        );
        let gone = which(ids);
        must(&mut g, StainEdit::Delete { node: gone });
        assert!(g.node(gone).is_none(), "the deleted node is still there");
        assert!(
            g.wires().iter().all(|w| w.from != gone && w.to != gone),
            "a wire of the deleted node is still there"
        );
        let got = colour_of(&gpu, &g);
        let grey = match combiner {
            "multiply" => bits(got) == bits([FTLE; 3]),
            _ => got
                .iter()
                .all(|&x| (f64::from(x) - f64::from(FTLE).powi(3)).abs() <= tol),
        };
        assert!(
            grey,
            "{combiner}: with the node deleted shade() draws {got:?}, not the greyscale of the brightness {FTLE}"
        );
    }
}

#[test]
fn qa_delete_the_colour_node_gives_the_greyscale_of_the_brightness() {
    check_delete(|ids| ids.colour);
}

negative_control!(
    qa_delete_the_colour_node_gives_the_greyscale_of_the_brightness,
    "the brightness deleted instead, the colour is drawn as it is, which is no grey",
    expected = "not the greyscale of the brightness",
    check_delete(|ids| ids.brightness)
);

// ── REQ-RENDER-072: every state deletion reaches renders ─────────────────────────────────────────────────────────

/// A full graph: a source of `n` into a colour, a source of `ftle` into a brightness and into the second post's
/// field in, Multiply, and two posts. With it, its six free nodes.
fn full() -> (StainGraph, Vec<NodeId>) {
    let (mut g, ids) = two_slot("multiply");
    let sources: Vec<NodeId> = g
        .nodes()
        .filter(|n| n.kind == NodeKind::Source)
        .map(|n| n.id)
        .collect();
    let ftle = *sources
        .iter()
        .find(|&&s| g.node(s).map(|n| &n.occupant) == Some(&field("ftle")))
        .expect("the ftle source");
    let first = add(&mut g, NodeKind::Post, custom(ROTATE));
    let second = add(&mut g, NodeKind::Post, custom(MASK));
    wire(&mut g, COMBINER, first, 0);
    wire(&mut g, first, second, 0);
    wire(&mut g, ftle, second, 1);
    wire(&mut g, second, OUT, 0);
    let free = [sources, vec![ids.colour, ids.brightness, first, second]].concat();
    (g, free)
}

/// From the full graph, every set of its six free nodes deleted, one node at a time: the fixed singletons refuse and
/// leave the graph as it was (render_gui_spec §4, §7); every other deletion is taken; and each of the 64 states
/// renders through `render` as a graph, its dangling in-ports None (§13), and draws a finite colour.
fn check_every_deletion_renders(render: Render) {
    let gpu = GpuHarness::new().expect("a GPU device");
    let (start, free) = full();
    assert_eq!(free.len(), 6);
    for fixed in [COMBINER, OUT] {
        let mut g = start.clone();
        assert!(
            edit(&mut g, StainEdit::Delete { node: fixed }).is_err(),
            "a fixed singleton was deleted"
        );
        assert_eq!(g, start, "a refused edit changed the graph");
    }
    let mut assembled: BTreeMap<String, Fragment> = BTreeMap::new();
    let mut drawn: BTreeMap<String, [f32; 3]> = BTreeMap::new();
    let mut state = |g: &StainGraph, what: &str| -> Fragment {
        let key = g
            .canonical()
            .unwrap_or_else(|e| panic!("{what}: no canonical form: {e}"))
            .text();
        if let Some(f) = assembled.get(&key) {
            return f.clone();
        }
        let f = rendered(g, render).unwrap_or_else(|e| panic!("{what}: {e}"));
        assembled.insert(key, f.clone());
        f
    };
    // Each set's graph is the graph of the set without its last node, with that node deleted: 63 deletions, each
    // state rendered where it is reached.
    let mut graphs: Vec<StainGraph> = vec![start.clone()];
    state(&start, "the full graph");
    for set in 1usize..(1 << free.len()) {
        let last = set.ilog2() as usize;
        let mut g = graphs[set & !(1 << last)].clone();
        must(&mut g, StainEdit::Delete { node: free[last] });
        let fragment = state(&g, &format!("set {set:06b}, node {} deleted", free[last].0));
        graphs.push(g);
        let colour = match drawn.get(&fragment.source) {
            Some(&c) => c,
            None => {
                let c = draw(&gpu, &fragment)
                    .unwrap_or_else(|e| panic!("set {set:06b}: the state did not draw: {e}"));
                drawn.insert(fragment.source.clone(), c);
                c
            }
        };
        assert!(
            colour.iter().all(|x| x.is_finite()),
            "set {set:06b}: the state draws {colour:?}"
        );
    }
    let states = graphs.len();
    // The full graph and the graph with every free node deleted are different pictures: the count below is no
    // count of one state seen 64 times.
    assert!(
        drawn.len() > 1,
        "every deletion draws one fragment: nothing was deleted"
    );
    eprintln!(
        "qa_every_deletion_renders: {states} states, {} distinct canonical forms assembled, {} distinct \
         fragments drawn",
        assembled.len(),
        drawn.len()
    );
}

#[test]
fn qa_every_deletion_renders() {
    check_every_deletion_renders(total);
}

negative_control!(
    qa_every_deletion_renders,
    "a renderer with no identity for a dangling in-port refuses the states that have one, which then do not render",
    expected = "a state did not render",
    check_every_deletion_renders(|nodes| {
        if nodes.iter().any(|n| n.kind != assemble::Kind::Combiner && n.inputs.contains(&None)) {
            return Err(AssembleError::Occupant(
                "the control's renderer takes no dangling in-port".into(),
            ));
        }
        let stain = assemble::Stain::new(nodes)?;
        Ok(Rendered {
            fragment: assemble::assemble(&stain, Tier::FULL)?,
            invalid: None,
        })
    })
);

// ── REQ-COL-025: at most 8 posts, in declaration order, with insert and reorder ──────────────────────────────────

/// A colour of `n` through the pass-through combiner: the graph a chain is built on.
fn chain_base() -> StainGraph {
    let mut g = StainGraph::new();
    let n = add(&mut g, NodeKind::Source, field("n"));
    let colour = add(&mut g, NodeKind::Colour, custom(COLOUR));
    wire(&mut g, n, colour, 0);
    wire(&mut g, colour, COMBINER, 0);
    g
}

/// A post holding `occupant` appended to the live chain by three edits: added, fed by what `OUT` read, and read by
/// `OUT`. Refused at any of them, the graph is as that edit found it.
fn push_post(g: &mut StainGraph, occupant: Occupant) -> Result<NodeId, Refusal> {
    let tail = g
        .wires()
        .iter()
        .find(|w| w.to == OUT && w.port == 0)
        .map(|w| w.from)
        .expect("OUT reads the chain");
    let before: Vec<NodeId> = g.nodes().map(|n| n.id).collect();
    edit(
        g,
        StainEdit::Add {
            kind: NodeKind::Post,
            occupant,
        },
    )?;
    let post = g
        .nodes()
        .map(|n| n.id)
        .find(|id| !before.contains(id))
        .expect("an accepted add adds a node");
    edit(
        g,
        StainEdit::Wire {
            from: tail,
            to: post,
            port: 0,
        },
    )?;
    edit(
        g,
        StainEdit::Wire {
            from: post,
            to: OUT,
            port: 0,
        },
    )?;
    Ok(post)
}

/// Posts appended to the chain one at a time, `last` of them: each before the last is taken, the chain growing by
/// one; the last is refused, the chain stays at 8 (colour_composition §4.2: "Bounded at ≤ 8"), and the graph still
/// renders, its draw the 8 posts in order.
fn check_bound(last: usize) {
    let gpu = GpuHarness::new().expect("a GPU device");
    let mut g = chain_base();
    let mut want = N;
    for k in 1..last {
        let (occupant, mirror): (&str, Mirror) = match k % 3 {
            0 => (ROTATE, rotate),
            1 => (QUARTER_G, quarter_g),
            _ => (HALVE_R, halve_r),
        };
        if let Err(why) = push_post(&mut g, custom(occupant)) {
            panic!("post {k} of the chain is refused: {why:?}");
        }
        want = mirror(want);
        assert_eq!(g.chain().len(), k, "the chain after post {k}");
    }
    let taken = push_post(&mut g, custom(ROTATE));
    assert!(
        taken.is_err(),
        "post {last} of the chain was not refused: the chain holds {}",
        g.chain().len()
    );
    assert_eq!(g.chain().len(), 8, "the chain after the refusal");
    let got = colour_of(&gpu, &g);
    assert_eq!(
        bits(got),
        bits(want),
        "the 8-post chain draws {got:?}, not its posts in declaration order, {want:?}"
    );
}

#[test]
fn qa_post_chain_takes_eight_posts_and_refuses_a_ninth() {
    check_bound(9);
}

negative_control!(
    qa_post_chain_takes_eight_posts_and_refuses_a_ninth,
    "the 8th post is within the bound, and is taken",
    expected = "post 8 of the chain was not refused",
    check_bound(8)
);

/// A chain of two posts, rotate then quarter-G; the second moved to `position`; then a third, halve-R, inserted
/// between the two. With `position` 0 the chain reads quarter-G, rotate, and then quarter-G, halve-R, rotate: each
/// draw is its chain's posts in order, so the reorder and the insert change the output accordingly.
fn check_reorder(position: u32) {
    let gpu = GpuHarness::new().expect("a GPU device");
    let mut g = chain_base();
    let first = push_post(&mut g, custom(ROTATE)).expect("the first post");
    let second = push_post(&mut g, custom(QUARTER_G)).expect("the second post");
    assert_eq!(g.chain(), [first, second]);
    let before = colour_of(&gpu, &g);
    assert_eq!(
        bits(before),
        bits(quarter_g(rotate(N))),
        "two posts draw {before:?}, not rotate then quarter-G"
    );

    // A move to the post's own place may be taken or refused; either way the chain is what it was.
    let _ = edit(
        &mut g,
        StainEdit::Reorder {
            post: second,
            position,
        },
    );
    let got = colour_of(&gpu, &g);
    let want = rotate(quarter_g(N));
    assert_ne!(bits(want), bits(before), "the two posts commute");
    assert_eq!(
        bits(got),
        bits(want),
        "the second post moved to the front: the chain draws {got:?}; reordering did not change the output \
         accordingly, to {want:?}"
    );
    assert_eq!(g.chain(), [second, first]);

    let third = add(&mut g, NodeKind::Post, custom(HALVE_R));
    wire(&mut g, second, third, 0);
    wire(&mut g, third, first, 0);
    assert_eq!(g.chain(), [second, third, first]);
    let got = colour_of(&gpu, &g);
    let want = rotate(halve_r(quarter_g(N)));
    assert_eq!(
        bits(got),
        bits(want),
        "a post inserted mid-chain: the chain draws {got:?}, not {want:?}"
    );
}

#[test]
fn qa_post_chain_reorder_and_insert_change_the_output() {
    check_reorder(0);
}

negative_control!(
    qa_post_chain_reorder_and_insert_change_the_output,
    "a post moved to its own place leaves the chain, and the output, as they were",
    expected = "reordering did not change the output accordingly",
    check_reorder(1)
);

// ── REQ-GUI-024: a source's one param, its field, sets its out-port's subtype ────────────────────────────────────

/// A source whose field is picked, each of `cases` in turn: it has no in-port (render_gui_spec §3.1) and one `field`
/// out-port, whose subtype is the picked field's (§5), and the in-port of the colour it feeds reads the same.
fn check_picking(cases: &[(&str, Subtype)]) {
    let mut g = StainGraph::new();
    let source = add(&mut g, NodeKind::Source, field("diffusion"));
    let colour = add(&mut g, NodeKind::Colour, custom(COLOUR));
    wire(&mut g, source, colour, 0);
    for (name, want) in cases {
        must(
            &mut g,
            StainEdit::Occupant {
                node: source,
                occupant: field(name),
            },
        );
        let ports = g.ports_of(source).unwrap_or_else(|e| panic!("{e}"));
        assert!(ports.inputs.is_empty(), "a source keeps no input");
        assert_eq!(ports.outputs.len(), 1, "a source has one out-port");
        assert_eq!(ports.outputs[0].ty, PortType::Field);
        assert_eq!(
            ports.outputs[0].subtype.as_ref(),
            Some(want),
            "picking `{name}` does not set the out-port's subtype"
        );
        let fed = g.ports_of(colour).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            fed.inputs[0].subtype.as_ref(),
            Some(want),
            "the colour's in-port does not read `{name}`'s subtype"
        );
    }
}

#[test]
fn qa_picking_a_vector_field_sets_the_out_port_subtype() {
    check_picking(&[
        ("n", Subtype::Vector),
        ("ftle", Subtype::Scalar),
        ("n", Subtype::Vector),
    ]);
    // The picked vector field is carried whole: a colour of its three lanes draws them.
    let gpu = GpuHarness::new().expect("a GPU device");
    let got = colour_of(&gpu, &chain_base());
    assert_eq!(bits(got), bits(N), "a source of `n` draws {got:?}");
}

negative_control!(
    qa_picking_a_vector_field_sets_the_out_port_subtype,
    "a scalar field picked is no vector",
    expected = "picking `ftle` does not set the out-port's subtype",
    check_picking(&[("ftle", Subtype::Vector)])
);
