//! qa's tests for TASK-M7-12 at the assembler (REQ-RENDER-071, REQ-GUI-024, REQ-RENDER-072, REQ-RENDER-081; R-813,
//! R-814), written from the requirements and their sources, render_gui_spec Part II §2, §3.1, §4, §5 and §13 and
//! colour_composition §4.1 and §4.2, not from the implementation. The edits (delete, reorder, the 9th post, picking a
//! field) are tested through the editor's graph in `crates/engine/tests/qa_TASK-M7-12.rs`.
//!
//! Every stain is a node list drawn through its assembled `shade()` on the GPU; no test reads the generated text.
//! - REQ-RENDER-071: with each of colour and brightness absent, and 0, 1 and 3 posts, the draw is colour_composition
//!   §4.1's Multiply row, the colour and the brightness read from two different sources, so that a slot wired to the
//!   other's source is seen, and each chain's draw is its posts applied in order to the draw without them.
//! - REQ-GUI-024: one source feeds five consumers (a colour, a brightness and three posts' field ins); a margin node,
//!   a colour or a post (R-813), has two `field` in-ports, and the built-in margin's colour is a function of
//!   `top1 − top2` that both inputs reach. Which map the margin is drawn through is not assumed.
//! - REQ-RENDER-072: node lists that are no graph each render, never an error or a panic, and all to one colour, the
//!   same at every pixel, that no graph here draws.
//! - REQ-RENDER-081: the fallback's displayed colour is not the both-None mid-grey's, the invalid hatch's two
//!   (`#9B00FF`, `#48FFFF`, REQ-COL-055) nor `#141418` (R-467). The fallback's own value is a proposal the human
//!   confirms at the M7 gate, so no test here names it; the OKLab distances are printed, with no floor of qa's.
//!
//! Exactness: the sample's values and the occupants' constants are exact in binary and every occupant is a product or
//! a swizzle, so a draw is compared with its mirror bit for bit. The one colour that goes through OKLab, the flat
//! mid-grey `OKLab(0.6, 0, 0)`, is compared in linear sRGB within REQ-COL-049's round-trip tolerance, read from
//! `fixtures/gates/oklab-roundtrip/gate.json` (provisional, R-182): an achromatic OKLab `(L, 0, 0)` is linear
//! `L³` on each channel, by Ottosson's matrices, whose rows from LMS sum to 1.
//!
//! Every check takes its subject as an argument, and its negative control runs it on a wrong one (R-176).

use std::path::Path;

use ledger::gen::prelude;
use render::assemble::{
    self, AssembleError, Fragment, Kind, Node, Occupant, PortType, Rendered, Stain, Tier, View,
};
use render::pipeline_cache::{block_layout, encode};
use validation::gpu::{BindingKind, GpuHarness};
use validation::negative_control;

// ── The draw ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// The sample a draw's entry fills.
#[derive(Clone, Copy, Debug)]
struct Sample {
    n: [f32; 3],
    ftle: f32,
    diffusion: f32,
}

const SAMPLE: Sample = Sample {
    n: [0.5, 0.25, 0.125],
    ftle: 0.75,
    diffusion: 0.375,
};

/// `fragment` drawn over a `side` × `side` target for the sample `s`: `shade()`'s linear RGB at each pixel, each
/// node's uniforms at their defaults.
fn draw_all(gpu: &GpuHarness, fragment: &Fragment, s: Sample, side: u32) -> Vec<[f32; 3]> {
    let entry = format!(
        "\n@fragment\nfn qa_shade(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<u32> {{\n    var ctx: Ctx;\n    \
         ctx.frag_xy = pos.xy;\n    ctx.sample.n = vec3<f32>({:?}, {:?}, {:?});\n    ctx.sample.ftle = {:?};\n    \
         ctx.sample.diffusion = {:?};\n    return vec4<u32>(bitcast<vec3<u32>>(shade(ctx)), 0u);\n}}\n",
        s.n[0], s.n[1], s.n[2], s.ftle, s.diffusion
    );
    let source = fragment.source.clone() + &entry;
    let mut kinds = vec![BindingKind::Uniform];
    kinds.extend(fragment.uniforms.iter().map(|_| BindingKind::Uniform));
    let kernel = gpu
        .fragment(&source, "qa_shade", &[&kinds], side, side)
        .unwrap_or_else(|e| panic!("the fragment does not compile to a draw: {e}"));
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
    let px = kernel
        .draw(&[&group])
        .unwrap_or_else(|e| panic!("the fragment does not draw: {e}"));
    assert_eq!(px.len(), (side * side) as usize, "one colour per pixel");
    px.iter()
        .map(|p| [p[0], p[1], p[2]].map(f32::from_bits))
        .collect()
}

/// One pixel of `fragment` for the sample `s`.
fn draw(gpu: &GpuHarness, fragment: &Fragment, s: Sample) -> [f32; 3] {
    draw_all(gpu, fragment, s, 1)[0]
}

fn assembled(nodes: &[Node]) -> Fragment {
    let stain = Stain::new(nodes.to_vec()).unwrap_or_else(|e| panic!("no graph: {e}"));
    assemble::assemble(&stain, Tier::FULL).unwrap_or_else(|e| panic!("no fragment: {e}"))
}

fn bits(c: [f32; 3]) -> [u32; 3] {
    c.map(f32::to_bits)
}

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

/// REQ-COL-049's round-trip tolerance, read from the gate's configuration, with its requirement checked.
fn tolerance() -> f64 {
    let config = validation::gate::config(root(), "oklab-roundtrip")
        .expect("the oklab-roundtrip gate config");
    assert_eq!(config.threshold.requirement, "REQ-COL-049");
    config.threshold.value
}

// ── The occupants ────────────────────────────────────────────────────────────────────────────────────────────────

fn node(kind: Kind, occupant: Occupant, inputs: &[Option<usize>]) -> Node {
    Node {
        kind,
        occupant,
        inputs: inputs.to_vec(),
    }
}

fn custom(text: &str) -> Occupant {
    Occupant::Custom(text.to_owned())
}

fn builtin(id: &str) -> Occupant {
    Occupant::BuiltIn(id.to_owned())
}

fn field(name: &str) -> Occupant {
    Occupant::Field(name.to_owned())
}

/// A colour of its field's three lanes, and a brightness of its field's first.
const COLOUR: &str = "fn colour(ctx: Ctx) -> vec3<f32> { return ctx.inputs[0].xyz; }";
const BRIGHTNESS: &str = "fn brightness(ctx: Ctx) -> f32 { return ctx.inputs[0].x; }";

/// Three posts, each exact in f32: a rotation of the channels and two scalings by a power of two. The rotation
/// commutes with neither scaling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Post {
    Rotate,
    QuarterG,
    HalveR,
}

impl Post {
    fn occupant(self) -> Occupant {
        custom(match self {
            Post::Rotate => "fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb.yzx; }",
            Post::QuarterG => {
                "fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb * vec3<f32>(1.0, 0.25, 1.0); }"
            }
            Post::HalveR => {
                "fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb * vec3<f32>(0.5, 1.0, 1.0); }"
            }
        })
    }

    fn apply(self, [r, g, b]: [f32; 3]) -> [f32; 3] {
        match self {
            Post::Rotate => [g, b, r],
            Post::QuarterG => [r, g * 0.25, b],
            Post::HalveR => [r * 0.5, g, b],
        }
    }
}

// ── REQ-RENDER-071: source(s) → colour? / brightness? → combiner → (post)* → OUT ─────────────────────────────────

/// The pipeline of render_gui_spec §2 as a node list: a source of `n` (0) and a source of `ftle` (1); the colour, fed
/// by `n`, where `colour`; the brightness, fed by source `brightness_from`, where `brightness`; the Multiply combiner;
/// `posts`, each fed by the node before it; and `OUT`.
fn pipeline(colour: bool, brightness: bool, posts: &[Post], brightness_from: usize) -> Vec<Node> {
    let mut g = vec![
        node(Kind::Source, field("n"), &[]),
        node(Kind::Source, field("ftle"), &[]),
    ];
    let c = colour.then(|| {
        g.push(node(Kind::Colour, custom(COLOUR), &[Some(0)]));
        g.len() - 1
    });
    let b = brightness.then(|| {
        g.push(node(
            Kind::Brightness,
            custom(BRIGHTNESS),
            &[Some(brightness_from)],
        ));
        g.len() - 1
    });
    g.push(node(Kind::Combiner, builtin("multiply"), &[c, b]));
    for p in posts {
        let before = g.len() - 1;
        g.push(node(Kind::Post, p.occupant(), &[Some(before)]));
    }
    let before = g.len() - 1;
    g.push(node(Kind::Out, Occupant::None, &[Some(before)]));
    g
}

/// Each of colour and brightness present and absent, with 0, 1 and 3 posts, the pipeline built by `build`: with no
/// post the draw is colour_composition §4.1's Multiply row (`C · B`; `C · 1`; `white · B`; the flat mid-grey
/// `OKLab(0.6, 0, 0)`), `C` the field `n` and `B` the field `ftle`; with posts, the draw is the posts applied, in
/// their order, to the draw without them.
fn check_pipeline(build: fn(bool, bool, &[Post]) -> Vec<Node>) {
    let gpu = GpuHarness::new().expect("a GPU device");
    let tol = tolerance();
    let (c, b) = (SAMPLE.n, SAMPLE.ftle);
    for (colour, brightness) in [(true, true), (true, false), (false, true), (false, false)] {
        let base = draw(&gpu, &assembled(&build(colour, brightness, &[])), SAMPLE);
        let row = match (colour, brightness) {
            (true, true) => Some(c.map(|x| x * b)),
            (true, false) => Some(c),
            (false, true) => Some([b; 3]),
            (false, false) => None,
        };
        match row {
            Some(want) => assert_eq!(
                bits(base),
                bits(want),
                "colour {colour}, brightness {brightness}: shade() draws {base:?}, not the truth table's {want:?}"
            ),
            None => {
                let grey = 0.6f64.powi(3);
                assert!(
                    base.iter().all(|&x| (f64::from(x) - grey).abs() <= tol),
                    "both None: shade() draws {base:?}, not the truth table's flat mid-grey, linear {grey}"
                );
            }
        }
        let chains: [&[Post]; 2] = [
            &[Post::QuarterG],
            &[Post::QuarterG, Post::Rotate, Post::HalveR],
        ];
        for posts in chains {
            let got = draw(&gpu, &assembled(&build(colour, brightness, posts)), SAMPLE);
            let want = posts.iter().fold(base, |x, p| p.apply(x));
            assert_eq!(
                bits(got),
                bits(want),
                "colour {colour}, brightness {brightness}, posts {posts:?}: shade() draws {got:?}, not the chain \
                 in order on the combiner's colour, {want:?}"
            );
        }
    }
}

#[test]
fn qa_wiring_each_slot_absent_and_each_chain_length_draws_the_pipeline() {
    check_pipeline(|colour, brightness, posts| pipeline(colour, brightness, posts, 1));
    // The chain's order is seen: the three posts in another order draw another colour.
    let forward = [Post::QuarterG, Post::Rotate, Post::HalveR]
        .iter()
        .fold(SAMPLE.n, |x, p| p.apply(x));
    let backward = [Post::HalveR, Post::Rotate, Post::QuarterG]
        .iter()
        .fold(SAMPLE.n, |x, p| p.apply(x));
    assert_ne!(forward, backward);
}

negative_control!(
    qa_wiring_each_slot_absent_and_each_chain_length_draws_the_pipeline,
    "a brightness wired to the colour's source is not the pipeline's: B is then n.x, not ftle",
    expected = "not the truth table's",
    check_pipeline(|colour, brightness, posts| pipeline(colour, brightness, posts, 0))
);

// ── REQ-GUI-024: fan-out to any number of consumers ──────────────────────────────────────────────────────────────

/// A colour of its field's first lane and two constants.
const TINT: &str =
    "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.inputs[0].x, 1.0, 0.25); }";
/// A post with a field in after its colour: the colour times the field.
const MASK: &str = "// @input mask
fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb * ctx.inputs[0].x; }";

/// One source of `ftle` (0) wired to five consumers: a colour (1), a brightness (2) and, after the Multiply combiner
/// (3), three masking posts (4, 5, 6), the last one's field in wired where `last`; then `OUT`.
fn fanned(last: bool) -> Vec<Node> {
    vec![
        node(Kind::Source, field("ftle"), &[]),
        node(Kind::Colour, custom(TINT), &[Some(0)]),
        node(Kind::Brightness, custom(BRIGHTNESS), &[Some(0)]),
        node(Kind::Combiner, builtin("multiply"), &[Some(1), Some(2)]),
        node(Kind::Post, custom(MASK), &[Some(3), Some(0)]),
        node(Kind::Post, custom(MASK), &[Some(4), Some(0)]),
        node(Kind::Post, custom(MASK), &[Some(5), last.then_some(0)]),
        node(Kind::Out, Occupant::None, &[Some(6)]),
    ]
}

/// `nodes`, [`fanned`]'s: the one source's value reaches each of its five consumers, so the draw is
/// `(f, 1, 0.25) · f · f · f · f`, `f` the field `ftle`.
fn check_fanout(nodes: Vec<Node>) {
    let sources = nodes.iter().filter(|n| n.kind == Kind::Source).count();
    assert_eq!(sources, 1, "the graph holds one source");
    let gpu = GpuHarness::new().expect("a GPU device");
    let f = SAMPLE.ftle;
    let got = draw(&gpu, &assembled(&nodes), SAMPLE);
    let want = [f, 1.0, 0.25].map(|x| x * f * f * f * f);
    assert_eq!(
        bits(got),
        bits(want),
        "one source into five consumers: shade() draws {got:?}, not {want:?}"
    );
}

#[test]
fn qa_fanout_one_source_feeds_five_consumers() {
    check_fanout(fanned(true));
}

negative_control!(
    qa_fanout_one_source_feeds_five_consumers,
    "with the fifth consumer's wire missing, the source's value does not reach it",
    expected = "one source into five consumers",
    check_fanout(fanned(false))
);

// ── REQ-GUI-024, R-813: a margin node has two field ins ──────────────────────────────────────────────────────────

/// A margin graph: sources of `ftle` (0) and `diffusion` (1), the colour `occupant` (2) with `top1` and `top2` its
/// two in-ports' wires, the pass-through combiner (3) and `OUT` (4).
fn margin_graph(occupant: &Occupant, top1: usize, top2: usize) -> Vec<Node> {
    vec![
        node(Kind::Source, field("ftle"), &[]),
        node(Kind::Source, field("diffusion"), &[]),
        node(Kind::Colour, occupant.clone(), &[Some(top1), Some(top2)]),
        node(Kind::Combiner, builtin("pass_through"), &[Some(2), None]),
        node(Kind::Out, Occupant::None, &[Some(3)]),
    ]
}

/// The margin node `occupant`, a colour (render_gui_spec §5; R-813): it has two `field` in-ports and no other; a
/// source has none (§3.1); and its colour is a function of the margin `top1 − top2` that both inputs reach: two
/// samples of one margin, 0.375, draw one colour, and the margins 0.375, 0 (both ports wired from the one source, §5:
/// fan-out is free) and −0.375 (the wires swapped) do not all draw the same.
fn check_margin(occupant: &Occupant) {
    let d = assemble::declaration(Kind::Colour, occupant).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        assemble::in_ports(Kind::Colour, &d),
        [PortType::Field, PortType::Field],
        "a margin node has two field in-ports"
    );
    let source =
        assemble::declaration(Kind::Source, &field("ftle")).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        assemble::in_ports(Kind::Source, &source),
        [],
        "a source keeps no input"
    );

    let gpu = GpuHarness::new().expect("a GPU device");
    let other = Sample {
        ftle: 0.5,
        diffusion: 0.125,
        ..SAMPLE
    };
    assert_eq!(SAMPLE.ftle - SAMPLE.diffusion, other.ftle - other.diffusion);
    let fragment = assembled(&margin_graph(occupant, 0, 1));
    let (one, two) = (draw(&gpu, &fragment, SAMPLE), draw(&gpu, &fragment, other));
    assert_eq!(
        bits(one),
        bits(two),
        "two samples of one margin draw {one:?} and {two:?}: the colour is no function of top1 − top2"
    );
    let zero = draw(&gpu, &assembled(&margin_graph(occupant, 0, 0)), SAMPLE);
    let swapped = draw(&gpu, &assembled(&margin_graph(occupant, 1, 0)), SAMPLE);
    assert!(
        one != zero || one != swapped,
        "the margins 0.375, 0 and −0.375 all draw {one:?}: the two inputs do not reach the colour"
    );
}

#[test]
fn qa_margin_node_has_two_field_ins_and_draws_their_difference() {
    check_margin(&builtin("margin"));
    // R-813: "a `colour` (or `post`) occupant declaring two `// @input`s". A post's two field ins follow its colour.
    let post = custom(
        "// @input top1\n// @input top2\nfn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb * (ctx.inputs[0].x \
         - ctx.inputs[1].x); }",
    );
    let d = assemble::declaration(Kind::Post, &post).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        assemble::in_ports(Kind::Post, &d),
        [PortType::Vec3, PortType::Field, PortType::Field]
    );
    let gpu = GpuHarness::new().expect("a GPU device");
    let nodes = vec![
        node(Kind::Source, field("ftle"), &[]),
        node(Kind::Source, field("diffusion"), &[]),
        node(Kind::Source, field("n"), &[]),
        node(Kind::Colour, custom(COLOUR), &[Some(2)]),
        node(Kind::Combiner, builtin("pass_through"), &[Some(3), None]),
        node(Kind::Post, post, &[Some(4), Some(0), Some(1)]),
        node(Kind::Out, Occupant::None, &[Some(5)]),
    ];
    let got = draw(&gpu, &assembled(&nodes), SAMPLE);
    let margin = SAMPLE.ftle - SAMPLE.diffusion;
    assert_eq!(bits(got), bits(SAMPLE.n.map(|x| x * margin)));
}

negative_control!(
    qa_margin_node_has_two_field_ins_and_draws_their_difference,
    "a colour of two inputs that draws the first alone is no margin",
    expected = "the colour is no function of top1 − top2",
    check_margin(&custom(
        "// @input top1\n// @input top2\nfn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.inputs[0].x); }"
    ))
);

// ── REQ-RENDER-072: an invalid graph shows a defined fallback, never a crash ─────────────────────────────────────

/// A graph: `n` as the colour, `ftle` as the brightness, Multiply.
fn graph() -> Vec<Node> {
    pipeline(true, true, &[], 1)
}

/// The graph with both slots None: the flat mid-grey.
fn both_none() -> Vec<Node> {
    vec![
        node(Kind::Combiner, builtin("multiply"), &[None, None]),
        node(Kind::Out, Occupant::None, &[Some(0)]),
    ]
}

/// Node lists that are no graph by render_gui_spec §4 and §6: the fixed singletons broken, a wire of the wrong type,
/// a wire to no node, a cycle, and in-ports a node does not have.
fn invalid_lists() -> Vec<(&'static str, Vec<Node>)> {
    let combiner = |inputs: &[Option<usize>]| node(Kind::Combiner, builtin("multiply"), inputs);
    let out = |from: usize| node(Kind::Out, Occupant::None, &[Some(from)]);
    let post = |from: usize| node(Kind::Post, Post::Rotate.occupant(), &[Some(from)]);
    vec![
        (
            "a wire from a node the list does not hold",
            vec![combiner(&[None, None]), out(7)],
        ),
        (
            "a source wired straight into OUT",
            vec![
                node(Kind::Source, field("n"), &[]),
                combiner(&[None, None]),
                out(0),
            ],
        ),
        (
            "a colour reading a colour",
            vec![
                node(Kind::Colour, custom(COLOUR), &[Some(1)]),
                node(Kind::Colour, custom(COLOUR), &[None]),
                combiner(&[Some(0), None]),
                out(2),
            ],
        ),
        (
            "a post reading itself",
            vec![combiner(&[None, None]), post(1), out(1)],
        ),
        (
            "a combiner of three in-ports",
            vec![combiner(&[None, None, None]), out(0)],
        ),
        (
            "an OUT with no in-port",
            vec![
                combiner(&[None, None]),
                node(Kind::Out, Occupant::None, &[]),
            ],
        ),
        ("two OUTs", vec![combiner(&[None, None]), out(0), out(0)]),
        (
            "an OUT alone",
            vec![node(Kind::Out, Occupant::None, &[None])],
        ),
        (
            "a brightness in the colour's place",
            vec![
                node(Kind::Brightness, custom(BRIGHTNESS), &[None]),
                combiner(&[Some(0), None]),
                out(1),
            ],
        ),
    ]
}

/// The side of the target the fallback's flatness is read over: 8 px, a whole period of REQ-COL-055's hatch (stripes
/// 4 px wide across x + y), so a pattern in the fallback's place is seen.
const SIDE: u32 = 8;

/// Every node list renders through `render`, in either view (render_gui_spec §13): a list that is no graph renders,
/// with the reason it is none, and all such lists draw one colour, the same at every pixel, which is neither the
/// colour a graph here draws nor the both-None mid-grey; a graph renders as itself, with no refusal.
fn check_invalid_lists_render(render: fn(Vec<Node>, View) -> Result<Rendered, AssembleError>) {
    let gpu = GpuHarness::new().expect("a GPU device");
    let graph_colour = draw(&gpu, &assembled(&graph()), SAMPLE);
    let grey = draw(&gpu, &assembled(&both_none()), SAMPLE);
    let mut drawn: Vec<(String, [f32; 3])> = Vec::new();
    for view in [View::Presentation, View::Debug] {
        for (what, nodes) in invalid_lists() {
            assert!(
                Stain::new(nodes.clone()).is_err(),
                "{what}: the list is a graph"
            );
            let rendered = render(nodes, view).unwrap_or_else(|e| {
                panic!("{what}, {view:?}: an invalid graph did not render: {e}")
            });
            assert!(
                rendered.invalid.is_some(),
                "{what}, {view:?}: rendered with no reason given"
            );
            let source = &rendered.fragment.source;
            let colour = match drawn.iter().find(|(s, _)| s == source) {
                Some(&(_, colour)) => colour,
                None => {
                    let pixels = draw_all(&gpu, &rendered.fragment, SAMPLE, SIDE);
                    assert!(
                        pixels.iter().all(|&p| bits(p) == bits(pixels[0])),
                        "{what}, {view:?}: the fallback is not flat: {pixels:?}"
                    );
                    drawn.push((source.clone(), pixels[0]));
                    pixels[0]
                }
            };
            assert!(
                colour.iter().all(|x| x.is_finite()),
                "{what}, {view:?}: the fallback draws {colour:?}"
            );
            assert_eq!(
                bits(colour),
                bits(drawn[0].1),
                "{what}, {view:?}: drawn {colour:?}, not the one defined fallback {:?}",
                drawn[0].1
            );
            assert!(
                colour != graph_colour && colour != grey,
                "{what}, {view:?}: the fallback {colour:?} is a colour a graph draws"
            );
        }
        let rendered =
            render(graph(), view).unwrap_or_else(|e| panic!("a graph did not render: {e}"));
        assert_eq!(rendered.invalid, None, "a graph was refused");
        if view == View::Presentation {
            assert_eq!(
                bits(draw(&gpu, &rendered.fragment, SAMPLE)),
                bits(graph_colour),
                "a graph does not render as itself"
            );
        }
    }
}

#[test]
fn qa_invalid_node_lists_render_one_flat_fallback() {
    check_invalid_lists_render(|nodes, view| {
        assemble::assemble_or_fallback(nodes, Tier::FULL, view)
    });
}

negative_control!(
    qa_invalid_node_lists_render_one_flat_fallback,
    "an assembler with no fallback refuses a list that is no graph, which then does not render",
    expected = "an invalid graph did not render",
    check_invalid_lists_render(|nodes, view| {
        let stain = Stain::new(nodes)?;
        Ok(Rendered {
            fragment: assemble::assemble_view(&stain, Tier::FULL, view)?,
            invalid: None,
        })
    })
);

// ── REQ-RENDER-081: the fallback is told from the colours it must not be read as ─────────────────────────────────

/// The sRGB transfer's inverse (IEC 61966-2-1), to 8 bits.
fn srgb8(linear: [f32; 3]) -> [u8; 3] {
    linear.map(|c| {
        let c = f64::from(c).clamp(0.0, 1.0);
        let e = if c <= 0.0031308 {
            12.92 * c
        } else {
            1.055 * c.powf(1.0 / 2.4) - 0.055
        };
        (e * 255.0).round() as u8
    })
}

/// OKLab of an 8-bit sRGB colour, through the crate's conversion: for the printed distances only.
fn oklab(c: [u8; 3]) -> [f64; 3] {
    render::present::linear_to_oklab(render::present::srgb8(c))
}

/// `fallback`, a fragment standing as the invalid graph's, drawn: its displayed colour, 8-bit sRGB, is none of the
/// colours REQ-RENDER-081 lists: the both-None flat mid-grey, as a graph with both slots None draws it; the invalid
/// hatch's violet `#9B00FF` and cyan `#48FFFF` (REQ-COL-055, R-428); the presentation view's `#141418` (R-467). The
/// OKLab distance to each is printed (`--nocapture`); how far is far enough is the proposal's, for the M7 gate.
fn check_told_apart(fallback: &Fragment) {
    let gpu = GpuHarness::new().expect("a GPU device");
    let colour = srgb8(draw(&gpu, fallback, SAMPLE));
    let grey = srgb8(draw(&gpu, &assembled(&both_none()), SAMPLE));
    let reserved = [
        ("the both-None mid-grey", grey),
        ("the invalid hatch's violet", [0x9b, 0x00, 0xff]),
        ("the invalid hatch's cyan", [0x48, 0xff, 0xff]),
        ("the presentation view's #141418", [0x14, 0x14, 0x18]),
    ];
    for (what, other) in reserved {
        let (a, b) = (oklab(colour), oklab(other));
        let d = (0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f64>().sqrt();
        eprintln!("the fallback {colour:02X?} to {what} {other:02X?}: OKLab distance {d:.4}");
        assert_ne!(
            colour, other,
            "the fallback {colour:02X?} is not told from {what}"
        );
    }
}

#[test]
fn qa_fallback_is_told_from_the_reserved_colours() {
    let rendered = assemble::assemble_or_fallback(Vec::new(), Tier::FULL, View::Presentation)
        .unwrap_or_else(|e| panic!("no node at all did not render: {e}"));
    assert!(rendered.invalid.is_some(), "no node at all is a graph");
    check_told_apart(&rendered.fragment);
}

negative_control!(
    qa_fallback_is_told_from_the_reserved_colours,
    "the both-None mid-grey in the fallback's place is not told from itself",
    expected = "is not told from the both-None mid-grey",
    check_told_apart(&assembled(&both_none()))
);
