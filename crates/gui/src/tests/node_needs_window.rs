//! What a node needs, read from its source, in the window: the inspector says what the selected node needs from the
//! run and, where the run lacks any of it, what is missing and why the node shows the invalid treatment instead, a
//! built-in and a custom alike; and the window writes a node's hidden measured range from the run's records, for a
//! node that needs one and no other. Each check also runs with the code under test broken, through `rejects`.
//!
//! Pass criteria, written before the runs (the real side over the zero view's 4 × 4 CPU records, which hold no FTLE
//! and no ensemble):
//! - `inspector`: Outcome × FTLE's lightness, selected, shows a "Needs from the run:" line naming the FTLE and a measured
//!   range, the FTLE's missing note in `Lack::says`'s words with why the node shows the invalid treatment, and that its
//!   range has no sample of ftle to measure; a custom colour reading the ensemble's spread shows the spread's missing
//!   note; Outcome × FTLE, which checks the FTLE, draws every pixel in the invalid treatment's `#141418` (control: a
//!   constant colour); the outcome palette, selected, shows no missing note. Controls: the lacks dropped; the needs not listed.
//! - `range`: with the `t_end_step` view as colour and a custom combiner whose `u_range` is a plain `vec2` it reads, the
//!   view holds `u_range` at the records' measured range of `t_end_step` (not its default), the combiner holds its own
//!   default, the undo depth is the graph's setting's alone, and the inspector of the view shows the range written.
//!   Controls: no range written; the range written by the uniform's name into every node declaring one; the range also
//!   set as a param through the history.

use engine::contract::render_state::{NodeId, NodeKind, Occupant, StainGraph};
use engine::contract::set_field::{Edit, RenderField, SetField};
use engine::contract::stain::assembled::node_source;
use engine::field::{Cpu, FieldBackend};

use super::mock_stain_preview::{depth, graph, on_stain, select};
use super::real_stain_drawn::plain_in;
use super::support::{frame_texts, rejects};
use crate::app::App;
use crate::capture::render_frame_on;
use crate::cli::real::open_capture;
use crate::headless::Headless;
use crate::real::{capture_view, RangesBroken, RealCanvas};
use crate::side::{EngineSide, RealSide};
use crate::stain::needs::{Broken, BROKEN};

const SIZE: [usize; 2] = [4, 4];

const COMBINER: NodeId = NodeId(0);

const FTLE_MISSING: &str = "Missing · needs the FTLE, which this run does not compute: it reads as not a number \
                            here, so the node's pixels show the invalid treatment where its code checks for that, and \
                            are not a measurement where it does not";

const SPREAD_MISSING: &str = "Missing · needs the ensemble's spread, which a run with no ensemble does not have: it \
                              reads as not a number here, so the node's pixels show the invalid treatment where its code \
                              checks for that, and are not a measurement where it does not";

const CONSTANT: &str = "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(0.5); }\n";

/// The invalid treatment's colour, 8-bit sRGB: `#141418`.
const INVALID: [u8; 3] = [0x14, 0x14, 0x18];

/// Asserts that every pixel of `canvas`'s plain draw of `g` is the invalid treatment's `#141418`.
fn check_invalid(canvas: &RealCanvas, g: StainGraph) {
    let rgba = plain_in(canvas, SIZE, g);
    assert!(!rgba.is_empty(), "a draw");
    for px in rgba.chunks(4) {
        assert_eq!(
            px[..3],
            INVALID,
            "a pixel of a node that checks what the run lacks"
        );
    }
}

const SPREAD: &str =
    "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.sample.ensemble_spread); }\n";

const READS_RANGE: &str = "// @uniform u_range: vec2<f32> = (0.25, 0.75)
fn combine(rgb: vec3<f32>, b: f32) -> vec3<f32> {
    return rgb * (uniforms.u_range.x + uniforms.u_range.y);
}
";

fn app() -> (App<RealSide>, Headless, std::sync::Arc<RealCanvas>) {
    let (side, canvas) = open_capture(&Cpu, SIZE).expect("the real side");
    let (app, h) = on_stain(App::new(side, crate::capture::FORMAT));
    (app, h, canvas)
}

/// Sets the graph `g` and draws it, once compiled and its nodes read: the undo depth the setting left.
fn show(app: &mut App<RealSide>, h: &mut Headless, canvas: &RealCanvas, g: StainGraph) -> u32 {
    app.set_field(SetField {
        edit: Edit::Render(RenderField::StainGraph(g)),
        no_history: false,
    });
    // The snapshot the depth is read from is taken at a frame; a frame paints nothing, so nothing is written yet.
    let _ = h.frame(app, Vec::new());
    let set = depth(app);
    app.side().recompile();
    app.side().wait_for_stain();
    let _ = render_frame_on(canvas, app, h);
    canvas.wait();
    let _ = render_frame_on(canvas, app, h);
    let _ = h.frame(app, Vec::new());
    set
}

fn node(g: &StainGraph, kind: NodeKind) -> NodeId {
    g.nodes().find(|n| n.kind == kind).expect("the node").id
}

fn preset(name: &str) -> StainGraph {
    engine::contract::stain::preset::library()
        .expect("the presets")
        .into_iter()
        .find(|p| p.name == name)
        .unwrap_or_else(|| panic!("no preset `{name}`"))
        .graph
}

/// The inspector's texts with node `id` selected.
fn inspected(app: &mut App<RealSide>, h: &mut Headless, id: NodeId) -> Vec<String> {
    select(app, h, id);
    let _ = h.frame(app, Vec::new());
    frame_texts(h, app)
}

fn check_inspector() {
    let (mut app, mut h, canvas) = app();
    let g = preset("Outcome × FTLE");
    let _ = show(&mut app, &mut h, &canvas, g.clone());
    let texts = inspected(&mut app, &mut h, node(&g, NodeKind::Brightness));
    let needs = texts
        .iter()
        .find(|t| t.starts_with("Needs from the run:"))
        .unwrap_or_else(|| panic!("no needs line in {texts:?}"));
    assert!(
        needs.contains("the FTLE") && needs.contains("a measured range"),
        "the lightness's needs: {needs}"
    );
    assert!(
        texts.iter().any(|t| t == FTLE_MISSING),
        "the FTLE's missing note in {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t
            == "Measured range · no sample of ftle to measure in this run: drawn at its default"),
        "the unmeasured range in {texts:?}"
    );
    let palette = node(&g, NodeKind::Colour);
    let texts = inspected(&mut app, &mut h, palette);
    assert!(
        !texts.iter().any(|t| t.starts_with("Missing")),
        "the outcome palette lacks nothing: {texts:?}"
    );
    let custom = engine::presets::view(SPREAD).expect("a view");
    let _ = show(&mut app, &mut h, &canvas, custom.clone());
    let texts = inspected(&mut app, &mut h, node(&custom, NodeKind::Colour));
    assert!(
        texts.iter().any(|t| t == SPREAD_MISSING),
        "the spread's missing note in {texts:?}"
    );
    check_invalid(&canvas, g);
    rejects("a node that reads nothing the run lacks", || {
        check_invalid(&canvas, engine::presets::view(CONSTANT).expect("a view"))
    });
}

fn broken(b: Broken, check: impl FnOnce()) {
    BROKEN.with(|c| c.set(b));
    let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    BROKEN.with(|c| c.set(Broken::Nothing));
    if let Err(e) = out {
        std::panic::resume_unwind(e);
    }
}

#[test]
fn the_inspector_says_what_a_node_needs_and_what_the_run_lacks() {
    check_inspector();
    rejects("the lacks dropped", || {
        broken(Broken::LacksDropped, check_inspector)
    });
    rejects("the needs not listed", || {
        broken(Broken::NeedsDropped, check_inspector)
    });
}

/// The `t_end_step` view into a combiner whose `u_range` is a plain `vec2` it reads.
fn ranged() -> StainGraph {
    let view = Occupant::Builtin("debug/generated/t_end_step".into());
    let text = node_source(NodeKind::Colour, &view)
        .expect("the view's source")
        .expect("a source");
    let mut g = engine::presets::view(&text).expect("a view");
    g.set_occupant(COMBINER, Occupant::Custom(READS_RANGE.into()))
        .expect("a combiner");
    g
}

/// `through_history`: the range also set as a param of the graph, as a write routed through `SetField` would.
fn check_range(ranges: RangesBroken, through_history: bool) {
    let (mut app, mut h, canvas) = app();
    *canvas.ranges_broken.lock().expect("the lock") = ranges;
    let g = ranged();
    let set = show(&mut app, &mut h, &canvas, g.clone());
    let (sim, _) = capture_view();
    let measured = Cpu
        .stored(&sim, sim.chart.window, SIZE)
        .expect("the records")
        .measured("t_end_step")
        .expect("a range of t_end_step");
    assert_ne!(measured, [0.0, 1.0], "the measured range is its default");
    let colour = node(&g, NodeKind::Colour);
    if through_history {
        let mut edited = graph(&app).clone();
        edited
            .set_param(colour, "u_range", measured.to_vec())
            .expect("a value of it");
        app.set_field(SetField {
            edit: Edit::Render(RenderField::StainGraph(edited)),
            no_history: false,
        });
        let _ = h.frame(&mut app, Vec::new());
    }
    assert_eq!(depth(&app), set, "the range write added no undo entry");
    assert_eq!(
        canvas.held(colour, "u_range"),
        Some(measured.to_vec()),
        "the view holds the measured range"
    );
    assert_eq!(
        canvas.held(COMBINER, "u_range"),
        Some(vec![0.25, 0.75]),
        "the combiner holds its own default"
    );
    let texts = inspected(&mut app, &mut h, colour);
    assert!(
        texts
            .iter()
            .any(|t| t
                .starts_with("Measured range · t_end_step over this run, on its view's scale: ")),
        "the range written, in {texts:?}"
    );
    assert_eq!(graph(&app), &g, "the graph is not edited by the write");
}

#[test]
fn the_window_writes_the_measured_range_only_where_a_node_needs_it() {
    check_range(RangesBroken::Nothing, false);
    rejects("no range written", || {
        check_range(RangesBroken::WritesNone, false)
    });
    rejects("the range written by name", || {
        check_range(RangesBroken::ByName, false)
    });
    rejects("the range written through the history", || {
        check_range(RangesBroken::Nothing, true)
    });
}
