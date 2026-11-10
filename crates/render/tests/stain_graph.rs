//! The stain graph to the fragment shader (TASK-M7-12; render_gui_spec Part II §2, §4, §5, §7, §13;
//! colour_composition §4.1, §4.2), each stain written as a node list (render never depends on engine,
//! systems_architecture §7.1) and drawn through the assembled `shade()` on the GPU:
//! - REQ-RENDER-071: graphs with each of colour and brightness absent and with 0, 1 and 3 posts assemble to a
//!   fragment whose `shade()` follows the wiring, statement by statement, and draws what the wiring gives
//!   (`stain_codegen_wiring_*`);
//! - REQ-GUI-027: deleting the colour node of a two-slot graph leaves the combiner's in-port dangling, which renders as
//!   None, the greyscale of the brightness; so do a deleted source and a deleted mid-chain post render by identity
//!   (`stain_delete_mid_node_*`);
//! - REQ-GUI-024: picking a vector field sets the source's subtype to vector; one source wired to a colour and a
//!   brightness node renders both; the margin node has two field in-ports (`stain_fanout_*`; R-813);
//! - REQ-COL-025: a 9th post in the chain is refused, the chain composes in declaration order, and reordering two
//!   posts or inserting one changes the output accordingly (`post_chain_order_*`);
//! - REQ-RENDER-072, REQ-RENDER-081: a node list that is no graph renders the defined fallback, never an error, and
//!   the fallback's proposed colour is distinguishable from the both-None mid-grey, the invalid hatch and `#141418`
//!   by its OKLab distance to each (`stain_fallback_*`; R-814). The engine's edit-sequence property is
//!   `crates/engine/tests/stain_always_renderable.rs`.
//!
//! The draws read no buffer: the test's entry fills the context's sample itself, `n = (0.25, 0.5, 0.75)`,
//! `ftle = 0.5` and `diffusion = 0.125`, values exact in binary, as are the posts' constants, so every draw but one
//! through OKLab is compared bit for bit with its mirror here. Every check takes its subject as an argument, and its
//! negative control runs it on a wrong one (R-176).

use ledger::gen::prelude;
use render::assemble::{
    self, AssembleError, Fragment, Kind, Node, Occupant, PortType, Rendered, Stain, Subtype, Tier,
    View,
};
use render::codegen::fallback;
use render::colour::combine::MID_GREY_L;
use render::colour::space;
use render::pipeline_cache::{block_layout, encode};
use render::present;
use validation::gpu::{BindingKind, GpuHarness};
use validation::negative_control;

// ── The draw ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// The sample the entry fills: `n`, `ftle` and `diffusion`.
const N: [f32; 3] = [0.25, 0.5, 0.75];
const FTLE: f32 = 0.5;
const DIFFUSION: f32 = 0.125;

/// The entry appended to an assembled stain: `shade()`'s colour at the pixel, its bits, for the sample above.
const ENTRY: &str = r"
@fragment
fn t_shade(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<u32> {
    var ctx: Ctx;
    ctx.frag_xy = pos.xy;
    ctx.sample.n = vec3<f32>(0.25, 0.5, 0.75);
    ctx.sample.ftle = 0.5;
    ctx.sample.diffusion = 0.125;
    return vec4<u32>(bitcast<vec3<u32>>(shade(ctx)), 0u);
}
";

/// `fragment` drawn: `shade()`'s linear RGB at one pixel, each node's uniforms at their defaults.
fn draw(gpu: &GpuHarness, fragment: &Fragment) -> [f32; 3] {
    let source = fragment.source.clone() + ENTRY;
    let mut kinds = vec![BindingKind::Uniform];
    kinds.extend(fragment.uniforms.iter().map(|_| BindingKind::Uniform));
    let kernel = gpu
        .fragment(&source, "t_shade", &[&kinds], 1, 1)
        .unwrap_or_else(|e| panic!("{e}"));
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
    let px = kernel.draw(&[&group]).unwrap_or_else(|e| panic!("{e}"));
    [px[0][0], px[0][1], px[0][2]].map(f32::from_bits)
}

fn stain(nodes: &[Node]) -> Stain {
    Stain::new(nodes.to_vec()).unwrap_or_else(|e| panic!("{e}"))
}

fn assembled(nodes: &[Node]) -> Fragment {
    assemble::assemble(&stain(nodes), Tier::FULL).unwrap_or_else(|e| panic!("{e}"))
}

/// The statements of `shade()`'s body in `source`, each trimmed and without its comment, down to its `return`.
fn shade_body(source: &str) -> Vec<String> {
    let start = "fn shade(ctx: Ctx) -> vec3<f32> {\n";
    let at = source.find(start).expect("shade()") + start.len();
    let body = &source[at..];
    let end = body.find("\n}\n").expect("shade()'s end");
    body[..end]
        .lines()
        .map(|l| l.split("//").next().unwrap_or_default().trim().to_owned())
        .collect()
}

/// The bound on a colour drawn through OKLab against its mirror, per OKLab axis: a few f32 ε of the GPU's evaluation,
/// as `tests/combiner.rs` bounds the same maps.
const TOL_LAB: f64 = 1e-6;

fn wide(x: [f32; 3]) -> [f64; 3] {
    x.map(f64::from)
}

/// Whether the linear colour `got` is OKLab `want` within [`TOL_LAB`] on each axis.
fn is_oklab(got: [f32; 3], want: [f64; 3]) -> bool {
    let lab = space::linear_to_oklab(wide(got));
    (0..3).all(|k| (lab[k] - want[k]).abs() <= TOL_LAB)
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

/// Three posts no two of which commute.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Post {
    Halve,
    Lift,
    Rotate,
}

impl Post {
    fn occupant(self) -> Occupant {
        custom(match self {
            Post::Halve => "fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb * 0.5; }",
            Post::Lift => {
                "fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb + vec3<f32>(0.125); }"
            }
            Post::Rotate => "fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb.yzx; }",
        })
    }

    /// The post's mirror, in f32.
    fn apply(self, [r, g, b]: [f32; 3]) -> [f32; 3] {
        match self {
            Post::Halve => [r * 0.5, g * 0.5, b * 0.5],
            Post::Lift => [r + 0.125, g + 0.125, b + 0.125],
            Post::Rotate => [g, b, r],
        }
    }
}

/// `posts` applied to `rgb` in order.
fn chained(rgb: [f32; 3], posts: &[Post]) -> [f32; 3] {
    posts.iter().fold(rgb, |c, p| p.apply(c))
}

/// The flat mid-grey, both slots None (colour_composition §4.1), linear, in f32.
fn mid_grey() -> [f32; 3] {
    present::ramp_grey(MID_GREY_L).map(|c| c as f32)
}

// ── REQ-RENDER-071: shade() follows the wiring ───────────────────────────────────────────────────────────────────

/// One wiring: whether a colour and a brightness feed the combiner, and the post chain.
#[derive(Clone, Debug)]
struct Wiring {
    colour: bool,
    brightness: bool,
    posts: Vec<Post>,
}

/// The wiring as a node list: one source of `n`, the colour and the brightness it feeds where the wiring has them, the
/// Multiply combiner, the posts each fed by the node before it, and `OUT`. With `posts` in place of the wiring's.
fn wiring_nodes(w: &Wiring, posts: &[Post]) -> Vec<Node> {
    let mut g = vec![node(Kind::Source, field("n"), &[])];
    let slot = |g: &mut Vec<Node>, live: bool, kind: Kind, text: &str| {
        live.then(|| {
            g.push(node(kind, custom(text), &[Some(0)]));
            g.len() - 1
        })
    };
    let c = slot(&mut g, w.colour, Kind::Colour, COLOUR);
    let b = slot(&mut g, w.brightness, Kind::Brightness, BRIGHTNESS);
    g.push(node(Kind::Combiner, builtin("multiply"), &[c, b]));
    for p in posts {
        let before = g.len() - 1;
        g.push(node(Kind::Post, p.occupant(), &[Some(before)]));
    }
    let before = g.len() - 1;
    g.push(node(Kind::Out, Occupant::None, &[Some(before)]));
    g
}

/// The statements `shade()` holds for the wiring, in the canonical numbering: the source where a live node reads it,
/// the colour, the brightness, the combiner by the truth table's row, each post in chain order, and `OUT`'s return.
fn wiring_statements(w: &Wiring) -> Vec<String> {
    /// Node `next`'s context, with its one field input wired to the source where `wired`; the node's number.
    fn context(out: &mut Vec<String>, next: &mut usize, wired: bool) -> usize {
        let i = *next;
        *next += 1;
        out.push(format!("var c{i} = ctx;"));
        if wired {
            out.push(format!("c{i}.inputs[0] = f0;"));
        }
        i
    }
    let mut out = Vec::new();
    let mut next = 0;
    if w.colour || w.brightness {
        out.push("let f0 = n0_source(ctx);".to_owned());
        next = 1;
    }
    if w.colour {
        let i = context(&mut out, &mut next, true);
        out.push(format!("let rgb = n{i}_colour(c{i});"));
    }
    if w.brightness {
        let i = context(&mut out, &mut next, true);
        out.push(format!("let b = n{i}_brightness(c{i});"));
    }
    let combiner = next;
    next += 1;
    out.push(match (w.colour, w.brightness) {
        (true, true) => format!("var out = n{combiner}_combine(rgb, b);"),
        (true, false) => "var out = rgb;".to_owned(),
        (false, true) => format!("var out = n{combiner}_combine(vec3<f32>(1.0), b);"),
        (false, false) => "var out = ramp_grey(0.6);".to_owned(),
    });
    for _ in &w.posts {
        let i = context(&mut out, &mut next, false);
        out.push(format!("out = n{i}_post(c{i}, out);"));
    }
    out.push("return out;".to_owned());
    out
}

/// What the wiring draws: the truth table's row under Multiply, on the entry's sample, then the posts in order.
fn wiring_colour(w: &Wiring) -> [f32; 3] {
    let combined = match (w.colour, w.brightness) {
        (true, true) => N.map(|c| c * N[0]),
        (true, false) => N,
        (false, true) => [N[0]; 3],
        (false, false) => mid_grey(),
    };
    chained(combined, &w.posts)
}

/// Each of colour and brightness present and absent, with 0, 1 and 3 posts, built by `nodes`: the assembled `shade()`
/// holds the wiring's statements and no other, and draws the wiring's colour, bit for bit, or within an f32 ε or two
/// of it where the combiner's output is the mid-grey, a constant the shader compiler may fold in another order.
fn check_wiring(nodes: fn(&Wiring) -> Vec<Node>) {
    let gpu = GpuHarness::new().expect("a GPU device");
    let chains: [&[Post]; 3] = [&[], &[Post::Lift], &[Post::Halve, Post::Lift, Post::Rotate]];
    for (colour, brightness) in [(true, true), (true, false), (false, true), (false, false)] {
        for posts in chains {
            let w = Wiring {
                colour,
                brightness,
                posts: posts.to_vec(),
            };
            let fragment = assembled(&nodes(&w));
            assert_eq!(
                shade_body(&fragment.source),
                wiring_statements(&w),
                "{w:?}: shade() does not follow the wiring"
            );
            let (got, want) = (draw(&gpu, &fragment), wiring_colour(&w));
            let near = (0..3).all(|k| (got[k] - want[k]).abs() <= 4.0 * f32::EPSILON);
            assert!(
                if colour || brightness {
                    got.map(f32::to_bits) == want.map(f32::to_bits)
                } else {
                    near
                },
                "{w:?}: shade() draws {got:?}; the wiring gives {want:?}"
            );
        }
    }
}

#[test]
fn stain_codegen_wiring_shade_follows_the_graph() {
    check_wiring(|w| wiring_nodes(w, &w.posts));
}

negative_control!(
    stain_codegen_wiring_shade_follows_the_graph,
    "a graph whose chain lacks its last post is not the wiring's",
    expected = "shade() does not follow the wiring",
    check_wiring(|w| wiring_nodes(w, &w.posts[..w.posts.len().saturating_sub(1)]))
);

/// The wiring's draw alone, on `wiring_colour`'s mirror given as `colour_of`: a control's hook for the draw.
fn check_wiring_draw(colour_of: fn(&Wiring) -> [f32; 3]) {
    let gpu = GpuHarness::new().expect("a GPU device");
    let w = Wiring {
        colour: true,
        brightness: true,
        posts: vec![Post::Halve, Post::Lift, Post::Rotate],
    };
    let got = draw(&gpu, &assembled(&wiring_nodes(&w, &w.posts)));
    assert_eq!(
        got.map(f32::to_bits),
        colour_of(&w).map(f32::to_bits),
        "{w:?}: shade() draws {got:?}, not the mirror's colour"
    );
}

#[test]
fn stain_codegen_wiring_draws_the_chain_in_order() {
    check_wiring_draw(wiring_colour);
}

negative_control!(
    stain_codegen_wiring_draws_the_chain_in_order,
    "the chain applied in reverse is another colour",
    expected = "not the mirror's colour",
    check_wiring_draw(|w| {
        let mut reversed = w.clone();
        reversed.posts.reverse();
        wiring_colour(&reversed)
    })
);

// ── REQ-GUI-027: a deleted mid-graph node's consumers read None ──────────────────────────────────────────────────

/// A two-slot graph: a source of `n` (0) into a colour (1) and a brightness (2), Replace-L (3), `OUT` (4).
fn two_slot() -> Vec<Node> {
    vec![
        node(Kind::Source, field("n"), &[]),
        node(Kind::Colour, custom(COLOUR), &[Some(0)]),
        node(Kind::Brightness, custom(BRIGHTNESS), &[Some(0)]),
        node(Kind::Combiner, builtin("replace_l"), &[Some(1), Some(2)]),
        node(Kind::Out, Occupant::None, &[Some(3)]),
    ]
}

/// `nodes` with node `i` deleted (render_gui_spec §7, "Delete node"): the node and its wires go, so each in-port it
/// fed dangles, and the later nodes move up one place.
fn deleted(nodes: &[Node], i: usize) -> Vec<Node> {
    let mut out = nodes.to_vec();
    out.remove(i);
    for n in &mut out {
        for input in &mut n.inputs {
            *input = match *input {
                Some(j) if j == i => None,
                Some(j) if j > i => Some(j - 1),
                other => other,
            };
        }
    }
    out
}

/// `nodes` with node `i` ghosted (colour_composition §4.1: "node "removal" sets None"): its occupant None, its wires
/// kept.
fn ghosted(nodes: &[Node], i: usize) -> Vec<Node> {
    let mut out = nodes.to_vec();
    out[i].occupant = Occupant::None;
    out
}

/// The colour node of the two-slot graph removed by `remove`: the graph still assembles, to the very source of the
/// graph that never had a colour, and draws the greyscale of the brightness, `OKLab(L = B, 0, 0)` with `B = n.x`.
fn check_delete_colour(remove: fn(&[Node], usize) -> Vec<Node>) {
    let gpu = GpuHarness::new().expect("a GPU device");
    let before = draw(&gpu, &assembled(&two_slot()));
    assert!(
        !is_oklab(before, [f64::from(N[0]), 0.0, 0.0]),
        "the two-slot graph is already grey: {before:?}"
    );
    let fragment = assembled(&remove(&two_slot(), 1));
    let got = draw(&gpu, &fragment);
    assert!(
        is_oklab(got, [f64::from(N[0]), 0.0, 0.0]),
        "the colour deleted: shade() draws OKLab {:?}, not the greyscale of the brightness ({}, 0, 0)",
        space::linear_to_oklab(wide(got)),
        N[0]
    );
    let never = vec![
        node(Kind::Source, field("n"), &[]),
        node(Kind::Brightness, custom(BRIGHTNESS), &[Some(0)]),
        node(Kind::Combiner, builtin("replace_l"), &[None, Some(1)]),
        node(Kind::Out, Occupant::None, &[Some(2)]),
    ];
    assert_eq!(
        fragment.source,
        assembled(&never).source,
        "the colour deleted is not the graph without a colour"
    );
}

#[test]
fn stain_delete_mid_node_the_colour_gives_the_greyscale_of_the_brightness() {
    check_delete_colour(deleted);
    check_delete_colour(ghosted);
}

negative_control!(
    stain_delete_mid_node_the_colour_gives_the_greyscale_of_the_brightness,
    "deleting the brightness instead keeps the colour as it is, which is no greyscale",
    expected = "not the greyscale of the brightness",
    check_delete_colour(|g, _| deleted(g, 2))
);

/// Other mid-graph deletions, each by `remove`, each still drawn by identity (render_gui_spec §13): the source, so the
/// colour's and the brightness's in-ports dangle and both are None, the flat mid-grey; the brightness, the colour as
/// it is; and the middle post of three, whose successor's colour in-port dangles and reads the combiner, the chain
/// before it empty.
fn check_delete_others(remove: fn(&[Node], usize) -> Vec<Node>) {
    let gpu = GpuHarness::new().expect("a GPU device");
    let got = draw(&gpu, &assembled(&remove(&two_slot(), 0)));
    assert!(
        is_oklab(got, [MID_GREY_L, 0.0, 0.0]),
        "the source deleted: shade() draws OKLab {:?}, not the mid-grey of both None",
        space::linear_to_oklab(wide(got))
    );
    let got = draw(&gpu, &assembled(&remove(&two_slot(), 2)));
    assert_eq!(
        got.map(f32::to_bits),
        N.map(f32::to_bits),
        "the brightness deleted: shade() draws {got:?}, not the colour as it is"
    );
    let w = Wiring {
        colour: true,
        brightness: false,
        posts: vec![Post::Halve, Post::Lift, Post::Rotate],
    };
    // The source 0, the colour 1, the combiner 2, the posts 3, 4 and 5.
    let got = draw(&gpu, &assembled(&remove(&wiring_nodes(&w, &w.posts), 4)));
    assert_eq!(
        got.map(f32::to_bits),
        Post::Rotate.apply(N).map(f32::to_bits),
        "the middle post deleted: shade() draws {got:?}, not the last post on the combiner's colour"
    );
}

#[test]
fn stain_delete_mid_node_every_deletion_renders_by_identity() {
    check_delete_others(deleted);
}

negative_control!(
    stain_delete_mid_node_every_deletion_renders_by_identity,
    "a deletion that removes nothing leaves the graph drawing as it did",
    expected = "the source deleted",
    check_delete_others(|g, _| g.to_vec())
);

// ── REQ-GUI-024: sources, fan-out and multi-source ───────────────────────────────────────────────────────────────

/// Picking a field sets the source's out-port subtype (render_gui_spec §5), `cases` each a field and its subtype; and
/// a source of the vector field carries its three lanes, which a colour reads and draws.
fn check_subtypes(cases: &[(&str, Subtype)]) {
    let fields = assemble::source_fields().expect("the fields");
    for &(name, want) in cases {
        assert!(
            fields.iter().any(|f| f == name),
            "`{name}` is no field a source picks"
        );
        assert_eq!(
            assemble::field_subtype(name),
            Ok(want),
            "picking `{name}` does not set the subtype {want:?}"
        );
    }
    let gpu = GpuHarness::new().expect("a GPU device");
    let w = Wiring {
        colour: true,
        brightness: false,
        posts: Vec::new(),
    };
    let got = draw(&gpu, &assembled(&wiring_nodes(&w, &[])));
    assert_eq!(
        got.map(f32::to_bits),
        N.map(f32::to_bits),
        "a source of the vector field `n` does not carry its three lanes"
    );
}

#[test]
fn stain_fanout_picking_a_vector_field_sets_subtype_vector() {
    check_subtypes(&[
        ("n", Subtype::Vector),
        ("ftle", Subtype::Scalar),
        ("state", Subtype::Categorical),
    ]);
}

negative_control!(
    stain_fanout_picking_a_vector_field_sets_subtype_vector,
    "the vector field read as a scalar is not its subtype",
    expected = "picking `n` does not set the subtype",
    check_subtypes(&[("n", Subtype::Scalar)])
);

/// One source wired to a colour and a brightness node, `nodes`: the canonical form holds the one source, feeding
/// both; `shade()` calls it once; and the draw is the colour times the brightness, both read from that source.
fn check_fanout(nodes: Vec<Node>) {
    let canonical = stain(&nodes).canonical();
    let form = canonical.stain().nodes();
    let sources: Vec<usize> = (0..form.len())
        .filter(|&i| form[i].kind == Kind::Source)
        .collect();
    assert_eq!(sources.len(), 1, "one source: {form:?}");
    for kind in [Kind::Colour, Kind::Brightness] {
        let fed = form
            .iter()
            .find(|n| n.kind == kind)
            .map(|n| n.inputs.clone());
        assert_eq!(
            fed,
            Some(vec![Some(sources[0])]),
            "the one source does not feed the {}",
            kind.name()
        );
    }
    let fragment = assemble::assemble(&stain(&nodes), Tier::FULL).unwrap_or_else(|e| panic!("{e}"));
    let calls = shade_body(&fragment.source)
        .iter()
        .filter(|l| l.contains("_source(ctx)"))
        .count();
    assert_eq!(calls, 1, "shade() reads the one source once");
    let gpu = GpuHarness::new().expect("a GPU device");
    let got = draw(&gpu, &fragment);
    assert_eq!(
        got.map(f32::to_bits),
        N.map(|c| c * N[0]).map(f32::to_bits),
        "one source into a colour and a brightness: shade() draws {got:?}, which does not render both"
    );
}

/// The fan-out: `two_slot`'s wiring under Multiply.
fn fanned() -> Vec<Node> {
    let mut g = two_slot();
    g[3].occupant = builtin("multiply");
    g
}

#[test]
fn stain_fanout_one_source_feeds_a_colour_and_a_brightness() {
    check_fanout(fanned());
}

negative_control!(
    stain_fanout_one_source_feeds_a_colour_and_a_brightness,
    "a source wired to the colour alone leaves the brightness dangling",
    expected = "the one source does not feed the brightness",
    check_fanout({
        let mut g = fanned();
        g[2].inputs = vec![None];
        g
    })
);

/// A margin graph: sources of `ftle` (0) and `diffusion` (1) into `occupant`'s in-ports `top1` and `top2`, a colour
/// (2), the pass-through combiner (3) and `OUT` (4).
fn margin_graph(occupant: &Occupant, top1: Option<usize>, top2: Option<usize>) -> Vec<Node> {
    vec![
        node(Kind::Source, field("ftle"), &[]),
        node(Kind::Source, field("diffusion"), &[]),
        node(Kind::Colour, occupant.clone(), &[top1, top2]),
        node(Kind::Combiner, builtin("pass_through"), &[Some(2), None]),
        node(Kind::Out, Occupant::None, &[Some(3)]),
    ]
}

/// The margin node `occupant`, a colour (R-813): it declares two field inputs, so its node has two `field` in-ports
/// and no third; each takes its own wire, and the draw is the margin `top1 − top2` on the grey ramp, which the two
/// wires swapped change; with one in-port dangling the node is None (render_gui_spec §13). A source keeps no input.
fn check_margin(occupant: &Occupant) {
    let d = assemble::declaration(Kind::Colour, occupant).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        assemble::in_ports(Kind::Colour, &d),
        [PortType::Field, PortType::Field],
        "a margin node has two field in-ports"
    );
    let names: Vec<&str> = d.inputs.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, ["top1", "top2"]);
    let mut three = margin_graph(occupant, Some(0), Some(1));
    three[2].inputs.push(Some(1));
    assert!(
        matches!(Stain::new(three), Err(AssembleError::Wire(_))),
        "a third wire attached to a node of two in-ports"
    );
    let sourced = "// @input a\nfn source(ctx: Ctx) -> Field { return Field(0.0, 0.0, 0.0, 0.0); }";
    assert!(
        matches!(
            assemble::declaration(Kind::Source, &custom(sourced)),
            Err(AssembleError::Occupant(_))
        ),
        "a source took an input"
    );

    let gpu = GpuHarness::new().expect("a GPU device");
    let margin = f64::from(FTLE - DIFFUSION);
    let got = draw(&gpu, &assembled(&margin_graph(occupant, Some(0), Some(1))));
    assert!(
        is_oklab(got, [margin, 0.0, 0.0]),
        "the margin draws OKLab {:?}, not top1 − top2 = {margin} on the grey ramp",
        space::linear_to_oklab(wide(got))
    );
    let got = draw(&gpu, &assembled(&margin_graph(occupant, Some(1), Some(0))));
    assert_eq!(
        got, [0.0; 3],
        "the wires swapped, top1 − top2 is below 0 and the ramp's black"
    );
    let dangling = stain(&margin_graph(occupant, Some(0), None));
    assert!(
        dangling
            .canonical()
            .stain()
            .nodes()
            .iter()
            .all(|n| n.kind != Kind::Colour),
        "a margin with one in-port dangling is None"
    );
}

#[test]
fn stain_fanout_a_margin_node_has_two_field_ins() {
    check_margin(&builtin("margin"));
}

negative_control!(
    stain_fanout_a_margin_node_has_two_field_ins,
    "a colour declaring one input is no margin node",
    expected = "a margin node has two field in-ports",
    check_margin(&custom(COLOUR))
);

// ── REQ-COL-025: the post chain ──────────────────────────────────────────────────────────────────────────────────

/// A colour of `n` through the pass-through combiner and the chain `posts`, each post `occupant_of` it.
fn chain_nodes(posts: &[Post], occupant_of: fn(usize, Post) -> Occupant) -> Vec<Node> {
    let mut g = vec![
        node(Kind::Source, field("n"), &[]),
        node(Kind::Colour, custom(COLOUR), &[Some(0)]),
        node(Kind::Combiner, builtin("pass_through"), &[Some(1), None]),
    ];
    for (k, &p) in posts.iter().enumerate() {
        let before = g.len() - 1;
        g.push(node(Kind::Post, occupant_of(k, p), &[Some(before)]));
    }
    let before = g.len() - 1;
    g.push(node(Kind::Out, Occupant::None, &[Some(before)]));
    g
}

/// Chains of 0 to 9 posts, the post at each place `occupant_of` it: up to the bound, 8 (colour_composition §4.2), the
/// chain is a stain, and the 8-post chain draws its posts in declaration order; a 9th post is refused as the chain's
/// bound.
fn check_bound(occupant_of: fn(usize, Post) -> Occupant) {
    assert_eq!(assemble::MAX_POSTS, 8);
    let posts: Vec<Post> = [Post::Halve, Post::Lift, Post::Rotate]
        .into_iter()
        .cycle()
        .take(9)
        .collect();
    for len in 0..=8 {
        let made = Stain::new(chain_nodes(&posts[..len], occupant_of));
        assert!(made.is_ok(), "a chain of {len} posts is refused: {made:?}");
    }
    let gpu = GpuHarness::new().expect("a GPU device");
    let got = draw(&gpu, &assembled(&chain_nodes(&posts[..8], occupant_of)));
    assert_eq!(
        got.map(f32::to_bits),
        chained(N, &posts[..8]).map(f32::to_bits),
        "the 8-post chain draws {got:?}, not its posts in declaration order"
    );
    let ninth = Stain::new(chain_nodes(&posts, occupant_of));
    assert!(
        matches!(ninth, Err(AssembleError::PostChain(_))),
        "a 9th post was not refused as the chain's bound: {ninth:?}"
    );
}

#[test]
fn post_chain_order_a_ninth_post_is_refused() {
    check_bound(|_, p| p.occupant());
}

negative_control!(
    post_chain_order_a_ninth_post_is_refused,
    "a 9th post that is None is no post of the live chain, and is taken",
    expected = "a 9th post was not refused",
    check_bound(|k, p| if k == 8 { Occupant::None } else { p.occupant() })
);

/// The chain's posts in `shade()`'s body, by the order of their calls: each call's node, in canonical numbering.
fn post_calls(fragment: &Fragment) -> Vec<String> {
    shade_body(&fragment.source)
        .into_iter()
        .filter(|l| l.contains("_post("))
        .collect()
}

/// Two posts, then the two reordered, then a third inserted between them, each chain built by `nodes`: every chain
/// draws its posts in its own order, so the reorder and the insert each change the output to the mirror's.
fn check_order(nodes: fn(&[Post]) -> Vec<Node>) {
    let gpu = GpuHarness::new().expect("a GPU device");
    let chains: [&[Post]; 3] = [
        &[Post::Halve, Post::Lift],
        &[Post::Lift, Post::Halve],
        &[Post::Halve, Post::Rotate, Post::Lift],
    ];
    let mut drawn = Vec::new();
    for posts in chains {
        let fragment = assembled(&nodes(posts));
        assert_eq!(
            post_calls(&fragment).len(),
            posts.len(),
            "{posts:?}: shade() does not call each post once"
        );
        let got = draw(&gpu, &fragment);
        assert_eq!(
            got.map(f32::to_bits),
            chained(N, posts).map(f32::to_bits),
            "{posts:?}: the chain draws {got:?}, not its posts in declaration order"
        );
        drawn.push(got);
    }
    assert_ne!(drawn[0], drawn[1], "reordering two posts changed nothing");
    assert_ne!(drawn[0], drawn[2], "inserting a post changed nothing");
}

#[test]
fn post_chain_order_reorder_and_insert_change_the_output() {
    check_order(|posts| chain_nodes(posts, |_, p| p.occupant()));
}

negative_control!(
    post_chain_order_reorder_and_insert_change_the_output,
    "a chain wired in one fixed order whatever its declaration is not the reordered chain",
    expected = "not its posts in declaration order",
    check_order(|posts| {
        let mut sorted = posts.to_vec();
        sorted.sort();
        chain_nodes(&sorted, |_, p| p.occupant())
    })
);

// ── REQ-RENDER-072, REQ-RENDER-081: the fallback ─────────────────────────────────────────────────────────────────

/// Whether a refusal is the one a case expects.
type Refusal = fn(&AssembleError) -> bool;

/// Node lists that are no graph, each with the refusal [`Stain::new`] gives it.
fn invalid_graphs() -> Vec<(&'static str, Vec<Node>, Refusal)> {
    let post = |from: usize| node(Kind::Post, Post::Halve.occupant(), &[Some(from)]);
    let combiner = || node(Kind::Combiner, builtin("multiply"), &[None, None]);
    let out = |from: usize| node(Kind::Out, Occupant::None, &[Some(from)]);
    vec![
        ("no node at all", Vec::new(), |e| {
            matches!(e, AssembleError::Singleton(_))
        }),
        ("no OUT", vec![combiner()], |e| {
            matches!(e, AssembleError::Singleton(_))
        }),
        ("two combiners", vec![combiner(), combiner(), out(1)], |e| {
            matches!(e, AssembleError::Singleton(_))
        }),
        ("no combiner", vec![out(0)], |e| {
            matches!(e, AssembleError::Cycle(_))
        }),
        (
            "a cycle between two posts",
            vec![combiner(), post(2), post(1), out(2)],
            |e| matches!(e, AssembleError::Cycle(_)),
        ),
        (
            "a brightness wired to OUT",
            vec![
                node(Kind::Brightness, Occupant::None, &[None]),
                combiner(),
                out(0),
            ],
            |e| matches!(e, AssembleError::Wire(_)),
        ),
        (
            "a colour wired past the combiner",
            vec![
                node(Kind::Colour, Occupant::None, &[None]),
                combiner(),
                out(0),
            ],
            |e| matches!(e, AssembleError::Backbone(_)),
        ),
        (
            "nine posts",
            chain_nodes(&[Post::Halve; 9], |_, p| p.occupant()),
            |e| matches!(e, AssembleError::PostChain(_)),
        ),
        (
            "the combiner None",
            vec![node(Kind::Combiner, Occupant::None, &[None, None]), out(0)],
            |e| matches!(e, AssembleError::Occupant(_)),
        ),
    ]
}

/// Every node list renders through `render`, never an error (render_gui_spec §13): a list that is no graph gives the
/// fallback's fragment, with its refusal, which draws the fallback's flat colour, bit for bit, in either view, with
/// no node function, no uniform block, no field and no debug arm; a graph gives its own assembly and no refusal.
fn check_fallback(render: fn(Vec<Node>, View) -> Result<Rendered, AssembleError>) {
    let gpu = GpuHarness::new().expect("a GPU device");
    let want = fallback::linear();
    for view in [View::Presentation, View::Debug] {
        for (what, nodes, refusal) in invalid_graphs() {
            assert!(Stain::new(nodes.clone()).is_err(), "{what} is a graph");
            let rendered = render(nodes, view)
                .unwrap_or_else(|e| panic!("{what}: an invalid graph did not render: {e}"));
            assert!(
                rendered.invalid.as_ref().is_some_and(refusal),
                "{what}: the refusal is {:?}",
                rendered.invalid
            );
            let f = &rendered.fragment;
            assert!(
                f.uniforms.is_empty() && f.fields.is_empty() && !f.debug_arm,
                "{what}: the fallback binds {:?}, reads {:?}, debug arm {}",
                f.uniforms,
                f.fields,
                f.debug_arm
            );
            let got = draw(&gpu, f);
            assert_eq!(
                got.map(f32::to_bits),
                want.map(f32::to_bits),
                "{what}, {view:?}: drawn {got:?}, not the fallback's colour {want:?}"
            );
        }
        let graph = two_slot();
        let rendered = render(graph.clone(), view).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(rendered.invalid, None, "a graph was refused");
        assert_eq!(
            Ok(rendered.fragment),
            assemble::assemble_view(&stain(&graph), Tier::FULL, view),
            "a graph does not render as its own assembly"
        );
    }
}

#[test]
fn stain_fallback_an_invalid_graph_renders_the_fallback() {
    check_fallback(|nodes, view| assemble::assemble_or_fallback(nodes, Tier::FULL, view));
    assert_eq!(
        assemble::fallback(Tier::FULL)
            .map(|f| shade_body(&f.source))
            .unwrap_or_else(|e| panic!("{e}")),
        ["var out = vec3<f32>(0.0, 1.0, 0.0);", "return out;"],
        "the fallback's shade() is its one statement"
    );
}

negative_control!(
    stain_fallback_an_invalid_graph_renders_the_fallback,
    "the both-None mid-grey in an invalid graph's place is not the fallback",
    expected = "not the fallback's colour",
    check_fallback(|nodes, view| {
        let rendered = assemble::assemble_or_fallback(nodes, Tier::FULL, view)?;
        if rendered.invalid.is_none() {
            return Ok(rendered);
        }
        let grey = stain(&[
            node(Kind::Combiner, builtin("multiply"), &[None, None]),
            node(Kind::Out, Occupant::None, &[Some(0)]),
        ]);
        Ok(Rendered {
            fragment: assemble::assemble(&grey, Tier::FULL)?,
            invalid: rendered.invalid,
        })
    })
);

/// The least OKLab distance proposed between the fallback and each colour it must be told from (R-71; REQ-RENDER-081:
/// the human confirms the fallback, and with it this floor, at the M7 gate).
const MIN_DISTANCE: f64 = 0.2;

/// The fallback colour `srgb8` against what it must be distinguishable from (render_gui_spec §13; R-467; R-814): the
/// both-None mid-grey `OKLab(0.6, 0, 0)`, the invalid hatch's two colours and the presentation view's `#141418`, each
/// by its Euclidean distance in OKLab, at least [`MIN_DISTANCE`]. Run with `--nocapture` for the distances.
fn check_distances(srgb8: [u8; 3]) {
    let lab = |linear: [f64; 3]| space::linear_to_oklab(linear);
    let colour = lab(present::srgb8(srgb8));
    let hatch = [
        present::debug_invalid([0.5, 0.5]),
        present::debug_invalid([4.5, 0.5]),
    ];
    assert_ne!(hatch[0], hatch[1], "the hatch's two colours");
    let others = [
        ("the both-None mid-grey", [MID_GREY_L, 0.0, 0.0]),
        ("the invalid hatch's first colour", lab(hatch[0])),
        ("the invalid hatch's second colour", lab(hatch[1])),
        (
            "the presentation view's #141418",
            lab(present::present_invalid()),
        ),
    ];
    for (what, other) in others {
        let d = (0..3)
            .map(|k| (colour[k] - other[k]).powi(2))
            .sum::<f64>()
            .sqrt();
        eprintln!("the fallback {srgb8:02X?} to {what}: OKLab distance {d:.4}");
        assert!(
            d >= MIN_DISTANCE,
            "the fallback {srgb8:02X?} is {d:.4} from {what} in OKLab, under {MIN_DISTANCE}"
        );
    }
}

#[test]
fn stain_fallback_colour_is_distinguishable() {
    check_distances(fallback::SRGB8);
    assert_eq!(fallback::SRGB8, [0x00, 0xff, 0x00]);
    assert_eq!(fallback::linear(), [0.0, 1.0, 0.0]);
}

negative_control!(
    stain_fallback_colour_is_distinguishable,
    "a flat grey fallback sits on the both-None mid-grey",
    expected = "from the both-None mid-grey in OKLab",
    check_distances([0x80, 0x80, 0x80])
);
