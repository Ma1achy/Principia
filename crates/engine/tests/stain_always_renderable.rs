//! The stain graph is always renderable (TASK-M7-12; render_gui_spec Part II §13, "Always-renderable"; §7;
//! REQ-RENDER-072; R-814, RQ-676 (b): the property runs here, in the crate that holds `StainGraph`):
//! - `stain_always_renderable_through_every_edit`, a property over random edit sequences (add, delete, duplicate,
//!   wire, unwire, reorder, pick an occupant), `validation::prop::CASES` cases (REQ-VAL-151): after every edit,
//!   accepted or refused, the graph lowers and assembles to a fragment, never the fallback and never an error, and
//!   each sequence's last graph draws on the GPU (a fragment drawn once is not drawn again); and that graph's node
//!   list, forced invalid (a second combiner, a second `OUT`, no `OUT`, `OUT` reading itself, or the combiner None),
//!   still renders: the defined fallback, with the refusal. The assemblies and the draws are the test's whole cost,
//!   and each is a function of its graph's canonical form, so the run's distinct forms are assembled and drawn once
//!   each, across threads, before the property runs with the same seed ([`warm`]); the property then asserts what
//!   it always did, of every state of every case;
//! - `stain_always_renderable_the_fallback_draws_its_colour`: the fallback's fragment draws the fallback's flat
//!   colour on the GPU, bit for bit.
//!
//! Dangling in-ports and None occupants render by identity (§13), so no edit sequence reaches the fallback: the
//! forced-invalid lists are written straight to the assembler, past the graph's own checks, which refuse them.
//! Each test registers its negative control (R-176).

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};

use engine::contract::set_field::{Edit, RenderField};
use engine::contract::stain::edit::{apply, StainEdit};
use engine::stain::{NodeId, NodeKind, Occupant, StainGraph};
use ledger::gen::prelude;
use proptest::prelude::*;
use render::assemble::{self, AssembleError, Fragment, Kind, Rendered, Stain, Tier, View};
use render::codegen::fallback;
use render::pipeline_cache::{block_layout, encode};
use validation::gpu::{BindingKind, GpuHarness};
use validation::negative_control;

// ── The draw ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// The entry appended to an assembled stain: `shade()`'s colour at the pixel, its bits, for a sample whose `n` and
/// `ftle` the entry fills; the draw reads no buffer.
const ENTRY: &str = r"
@fragment
fn t_shade(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<u32> {
    var ctx: Ctx;
    ctx.frag_xy = pos.xy;
    ctx.sample.n = vec3<f32>(0.25, 0.5, 0.75);
    ctx.sample.ftle = 0.5;
    return vec4<u32>(bitcast<vec3<u32>>(shade(ctx)), 0u);
}
";

/// `fragment` drawn: `shade()`'s linear RGB at one pixel, each node's uniforms at their defaults; or why it did not
/// draw.
fn draw(gpu: &GpuHarness, fragment: &Fragment) -> Result<[f32; 3], String> {
    let source = fragment.source.clone() + ENTRY;
    let mut kinds = vec![BindingKind::Uniform];
    kinds.extend(fragment.uniforms.iter().map(|_| BindingKind::Uniform));
    let kernel = gpu
        .fragment(&source, "t_shade", &[&kinds], 1, 1)
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

// ── The edits ────────────────────────────────────────────────────────────────────────────────────────────────────

fn custom(text: &str) -> Occupant {
    Occupant::Custom(text.to_owned())
}

fn builtin(id: &str) -> Occupant {
    Occupant::Builtin(id.to_owned())
}

const COLOUR: &str = "fn colour(ctx: Ctx) -> vec3<f32> { return ctx.inputs[0].xyz; }";
const BRIGHTNESS: &str = "fn brightness(ctx: Ctx) -> f32 { return ctx.inputs[0].x; }";
const HALVE: &str = "fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb * 0.5; }";
const LIFT: &str =
    "fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb + vec3<f32>(0.125); }";
/// A post with a field input after its colour.
const MASK: &str = "// @input mask
fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb * ctx.inputs[0].x; }";

/// What an edit adds or picks: every kind, each with occupants that compile, None among them, and the margin, a
/// colour of two field inputs.
fn palette() -> Vec<(NodeKind, Occupant)> {
    vec![
        (NodeKind::Source, Occupant::Field("n".into())),
        (NodeKind::Source, Occupant::Field("ftle".into())),
        (NodeKind::Colour, custom(COLOUR)),
        (NodeKind::Colour, builtin("margin")),
        (NodeKind::Colour, Occupant::None),
        (NodeKind::Brightness, custom(BRIGHTNESS)),
        (NodeKind::Brightness, Occupant::None),
        (NodeKind::Post, custom(HALVE)),
        (NodeKind::Post, custom(LIFT)),
        (NodeKind::Post, custom(MASK)),
        (NodeKind::Post, Occupant::None),
        (NodeKind::Combiner, builtin("multiply")),
        (NodeKind::Combiner, builtin("replace_l")),
        (NodeKind::Combiner, builtin("pass_through")),
        (NodeKind::Combiner, Occupant::None),
        (NodeKind::Out, Occupant::None),
    ]
}

/// One drawn edit: its nodes picked among the graph's by index, its occupant from the [`palette`], for `Pick` among
/// those of the picked node's kind.
#[derive(Clone, Copy, Debug)]
enum Op {
    Add(u8),
    Delete(u8),
    Duplicate(u8),
    Wire(u8, u8, u8),
    Unwire(u8, u8),
    Reorder(u8, u8),
    Pick(u8, u8),
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        any::<u8>().prop_map(Op::Add),
        any::<u8>().prop_map(Op::Delete),
        any::<u8>().prop_map(Op::Duplicate),
        (any::<u8>(), any::<u8>(), 0u8..3).prop_map(|(a, b, p)| Op::Wire(a, b, p)),
        (any::<u8>(), 0u8..3).prop_map(|(a, p)| Op::Unwire(a, p)),
        (any::<u8>(), 0u8..8).prop_map(|(a, p)| Op::Reorder(a, p)),
        (any::<u8>(), any::<u8>()).prop_map(|(a, o)| Op::Pick(a, o)),
    ]
}

/// The graph a sequence starts from: a source of `n` into a colour and a brightness, Multiply, two posts and `OUT`,
/// so that the first deletions and unwires already leave in-ports dangling.
fn start() -> StainGraph {
    let mut g = StainGraph::new();
    let (combiner, out) = (NodeId(0), NodeId(1));
    let s = g
        .add(NodeKind::Source, Occupant::Field("n".into()))
        .expect("source");
    let c = g.add(NodeKind::Colour, custom(COLOUR)).expect("colour");
    let b = g
        .add(NodeKind::Brightness, custom(BRIGHTNESS))
        .expect("brightness");
    let p1 = g.add(NodeKind::Post, custom(HALVE)).expect("post");
    let p2 = g.add(NodeKind::Post, custom(LIFT)).expect("post");
    g.set_occupant(combiner, builtin("multiply"))
        .expect("combiner");
    for (from, to, port) in [
        (s, c, 0),
        (s, b, 0),
        (c, combiner, 0),
        (b, combiner, 1),
        (combiner, p1, 0),
        (p1, p2, 0),
        (p2, out, 0),
    ] {
        g.connect(from, to, port).expect("a backbone wire");
    }
    g
}

/// `op` as an edit of `g`.
fn edit_of(g: &StainGraph, op: Op) -> StainEdit {
    let ids: Vec<NodeId> = g.nodes().map(|n| n.id).collect();
    let pick = |i: u8| ids[i as usize % ids.len()];
    let palette = palette();
    let from_palette = |o: u8| palette[o as usize % palette.len()].clone();
    match op {
        Op::Add(o) => {
            let (kind, occupant) = from_palette(o);
            StainEdit::Add { kind, occupant }
        }
        Op::Delete(a) => StainEdit::Delete { node: pick(a) },
        Op::Duplicate(a) => StainEdit::Duplicate { node: pick(a) },
        Op::Wire(a, b, p) => StainEdit::Wire {
            from: pick(a),
            to: pick(b),
            port: u32::from(p),
        },
        Op::Unwire(a, p) => StainEdit::Unwire {
            to: pick(a),
            port: u32::from(p),
        },
        Op::Reorder(a, p) => StainEdit::Reorder {
            post: pick(a),
            position: u32::from(p),
        },
        Op::Pick(a, o) => {
            // An occupant of the node's own kind: another kind's custom text would be a node that does not compile,
            // the custom occupant's failure and its last-valid path (gui_state_contract §5), not this property's.
            let node = pick(a);
            let kind = g.node(node).map(|n| n.kind);
            let options: Vec<Occupant> = palette
                .iter()
                .filter(|(k, _)| Some(*k) == kind)
                .map(|(_, occupant)| occupant.clone())
                .collect();
            StainEdit::Occupant {
                node,
                occupant: options[o as usize % options.len()].clone(),
            }
        }
    }
}

// ── The property ─────────────────────────────────────────────────────────────────────────────────────────────────

/// What renders a node list: the assembler's total entry point, or a control's stand-in.
type Render = fn(Vec<assemble::Node>) -> Result<Rendered, AssembleError>;

/// `nodes`, a graph's, forced invalid in one of five ways, by `how`: a second combiner; a second `OUT`; no `OUT`, a
/// None post in its place; `OUT` reading itself; the combiner None.
fn forced_invalid(mut nodes: Vec<assemble::Node>, how: u8) -> Vec<assemble::Node> {
    let at = |nodes: &[assemble::Node], kind: Kind| {
        nodes
            .iter()
            .position(|n| n.kind == kind)
            .expect("a graph holds a combiner and an OUT")
    };
    let extra = |kind: Kind, occupant: assemble::Occupant, inputs: usize| assemble::Node {
        kind,
        occupant,
        inputs: vec![None; inputs],
    };
    match how % 5 {
        0 => nodes.push(extra(
            Kind::Combiner,
            assemble::Occupant::BuiltIn("multiply".into()),
            2,
        )),
        1 => nodes.push(extra(Kind::Out, assemble::Occupant::None, 1)),
        2 => {
            let out = at(&nodes, Kind::Out);
            nodes[out].kind = Kind::Post;
        }
        3 => {
            let out = at(&nodes, Kind::Out);
            nodes[out].inputs = vec![Some(out)];
        }
        _ => {
            let combiner = at(&nodes, Kind::Combiner);
            nodes[combiner].occupant = assemble::Occupant::None;
        }
    }
    nodes
}

/// The fragments already assembled, by the canonical form's text, which the assembly is a function of at one tier
/// and view: a state seen before is not assembled again.
type Seen = RefCell<BTreeMap<String, Fragment>>;

/// `g` rendered through `render`: its fragment, never the fallback and never an error.
fn rendered(g: &StainGraph, render: Render, seen: &Seen) -> Result<Fragment, String> {
    let stain = g
        .lower()
        .map_err(|e| format!("the graph does not lower: {e}"))?;
    let key = stain.canonical().text();
    if let Some(f) = seen.borrow().get(&key) {
        return Ok(f.clone());
    }
    let r = render(stain.nodes().to_vec()).map_err(|e| format!("a state did not render: {e}"))?;
    if let Some(why) = r.invalid {
        return Err(format!(
            "a state an edit reached rendered the fallback: {why}"
        ));
    }
    seen.borrow_mut().insert(key, r.fragment.clone());
    Ok(r.fragment)
}

/// The graphs a sequence passes through: [`start`], then the graph after each of `ops`, an edit the graph refuses
/// leaving it as it was; each with the edit that led to it.
fn states(ops: &[Op]) -> Result<Vec<(Option<StainEdit>, StainGraph)>, String> {
    let mut g = start();
    let mut out = vec![(None, g.clone())];
    for &op in ops {
        let edit = edit_of(&g, op);
        if let Ok(set) = apply(&g, &edit) {
            g = match set.edit {
                Edit::Render(RenderField::StainGraph(next)) => next,
                _ => {
                    return Err(format!(
                        "{edit:?} wrote something other than the stain graph"
                    ))
                }
            };
        }
        out.push((Some(edit), g.clone()));
    }
    Ok(out)
}

/// One sequence: from [`start`], each of `ops` applied where the graph accepts it, the graph rendering after every
/// one ([`rendered`]); then the last graph drawn on the GPU, and its node list forced invalid by `how`, which the
/// graph's own check refuses and `render` still renders, as `fallback`'s fragment, with the refusal.
fn run_ops(
    ops: &[Op],
    how: u8,
    render: Render,
    gpu: &GpuHarness,
    fallback: &Fragment,
    seen: &Seen,
    drawn: &RefCell<BTreeSet<String>>,
) -> Result<(), String> {
    let states = states(ops)?;
    let mut last = None;
    for (edit, g) in &states {
        let fragment = rendered(g, render, seen).map_err(|e| match edit {
            Some(edit) => format!("after {edit:?}: {e}"),
            None => e,
        })?;
        last = Some((g, fragment));
    }
    let (g, fragment) = last.ok_or("a sequence holds its start")?;
    // A fragment drawn before draws the same again: the draw is a function of its source.
    if !drawn.borrow().contains(&fragment.source) {
        draw(gpu, &fragment).map_err(|e| format!("the last graph did not draw: {e}"))?;
        drawn.borrow_mut().insert(fragment.source);
    }

    let nodes = forced_invalid(g.lower().map_err(|e| e.to_string())?.nodes().to_vec(), how);
    if Stain::new(nodes.clone()).is_ok() {
        return Err(format!(
            "forced invalid by {how}, the list is still a graph"
        ));
    }
    let r = render(nodes).map_err(|e| format!("a forced-invalid graph did not render: {e}"))?;
    if r.invalid.is_none() || &r.fragment != fallback {
        return Err(format!(
            "a forced-invalid graph did not render the fallback: refusal {:?}",
            r.invalid
        ));
    }
    Ok(())
}

/// The property's assemblies and draws, done ahead of it and across threads: the property is run once over the same
/// cases (`seed`'s) to collect each state's canonical form and node list, with no assembly, then each distinct form is
/// assembled through `render`, and drawn where it is some sequence's last graph, on as many threads as the machine
/// gives. What succeeds is kept, in `seen` and `drawn`, as the property keeps it itself; whatever fails here, a
/// state that does not lower, an assembly that errs or gives the fallback, a draw that errs, is kept nowhere, so the
/// property meets it again in its case and fails there, with the edit and proptest's minimal case. Nothing the
/// property asserts is asserted here in its place: this only fills the two caches.
fn warm(
    seed: u64,
    strategy: &impl Strategy<Value = (Vec<Op>, u8)>,
    render: Render,
    gpu: &GpuHarness,
    seen: &Seen,
    drawn: &RefCell<BTreeSet<String>>,
) {
    // Each distinct form: its node list, and whether some sequence ends on it.
    let forms: RefCell<BTreeMap<String, (Vec<assemble::Node>, bool)>> = RefCell::default();
    let _ = validation::prop::check_with_seed(seed, strategy, |(ops, _)| {
        let Ok(states) = states(&ops) else {
            return Ok(());
        };
        for (i, (_, g)) in states.iter().enumerate() {
            let Ok(stain) = g.lower() else { continue };
            let mut forms = forms.borrow_mut();
            let form = forms
                .entry(stain.canonical().text())
                .or_insert_with(|| (stain.nodes().to_vec(), false));
            form.1 |= i + 1 == states.len();
        }
        Ok(())
    });
    let forms: Vec<(String, Vec<assemble::Node>, bool)> = forms
        .into_inner()
        .into_iter()
        .map(|(key, (nodes, last))| (key, nodes, last))
        .collect();
    let next = AtomicUsize::new(0);
    let done: Mutex<Vec<(String, Fragment, bool)>> = Mutex::default();
    let threads = std::thread::available_parallelism().map_or(1, usize::from);
    std::thread::scope(|scope| {
        for _ in 0..threads.min(forms.len()) {
            scope.spawn(|| {
                while let Some((key, nodes, last)) = forms.get(next.fetch_add(1, Ordering::Relaxed))
                {
                    let Ok(r) = render(nodes.clone()) else {
                        continue;
                    };
                    if r.invalid.is_some() {
                        continue;
                    }
                    let drew = *last && draw(gpu, &r.fragment).is_ok();
                    done.lock().unwrap_or_else(PoisonError::into_inner).push((
                        key.clone(),
                        r.fragment,
                        drew,
                    ));
                }
            });
        }
    });
    for (key, fragment, drew) in done.into_inner().unwrap_or_else(PoisonError::into_inner) {
        if drew {
            drawn.borrow_mut().insert(fragment.source.clone());
        }
        seen.borrow_mut().insert(key, fragment);
    }
}

/// The property, over `render`; with `ahead`, its assemblies and draws are done ahead of it ([`warm`]), which changes
/// how long it takes and nothing it asserts. A control, which fails in its first case, runs without.
fn check_always_renderable(render: Render, ahead: bool) {
    let gpu = GpuHarness::new().expect("a GPU device");
    let fallback = assemble::fallback(Tier::FULL).unwrap_or_else(|e| panic!("{e}"));
    let seen = Seen::default();
    let drawn = RefCell::new(BTreeSet::new());
    let strategy = (prop::collection::vec(op(), 1..24), any::<u8>());
    // One seed for both passes, so the forms assembled ahead are the property's own.
    let seed = validation::prop::seed();
    if ahead {
        warm(seed, &strategy, render, &gpu, &seen, &drawn);
    }
    let (ahead_forms, ahead_draws) = (seen.borrow().len(), drawn.borrow().len());
    let result = validation::prop::check_with_seed(seed, &strategy, |(ops, how)| {
        run_ops(&ops, how, render, &gpu, &fallback, &seen, &drawn).map_err(TestCaseError::fail)
    });
    eprintln!(
        "stain_always_renderable: {} distinct canonical forms assembled and {} distinct last graphs drawn over {} \
         cases ({ahead_forms} and {ahead_draws} of them ahead of the property)",
        seen.borrow().len(),
        drawn.borrow().len(),
        validation::prop::CASES
    );
    if let Err(failure) = result {
        eprintln!("{failure}");
        panic!("{failure}");
    }
}

#[test]
fn stain_always_renderable_through_every_edit() {
    check_always_renderable(
        |nodes| assemble::assemble_or_fallback(nodes, Tier::FULL, View::Presentation),
        true,
    );
}

negative_control!(
    stain_always_renderable_through_every_edit,
    "an assembler with no fallback refuses a forced-invalid graph, which then does not render",
    expected = "a forced-invalid graph did not render",
    check_always_renderable(
        |nodes| {
            let stain = Stain::new(nodes)?;
            Ok(Rendered {
                fragment: assemble::assemble(&stain, Tier::FULL)?,
                invalid: None,
            })
        },
        false
    )
);

/// The fallback's fragment drawn: `colour`, bit for bit.
fn check_fallback_draw(colour: [f32; 3]) {
    let gpu = GpuHarness::new().expect("a GPU device");
    let fallback = assemble::fallback(Tier::FULL).unwrap_or_else(|e| panic!("{e}"));
    let got = draw(&gpu, &fallback).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        got.map(f32::to_bits),
        colour.map(f32::to_bits),
        "the fallback draws {got:?}, not its colour {colour:?}"
    );
}

#[test]
fn stain_always_renderable_the_fallback_draws_its_colour() {
    check_fallback_draw(fallback::linear());
}

negative_control!(
    stain_always_renderable_the_fallback_draws_its_colour,
    "the fallback is not black",
    expected = "the fallback draws",
    check_fallback_draw([0.0; 3])
);
