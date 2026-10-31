//! `mock_stain_preview` (REQ-GUI-175, REQ-GUI-128; render_gui_spec Part II §1, §9; §G1): on the mock, an edit made in
//! the Stain editor's inspector is one `SetField` of the graph — shown in the next snapshot, one step of undo, logged —
//! and the preview, drawn from the graph as it stands, changes with it and back with its undo; a slider's drag is one
//! `SetField` when it ends (R-96); Recompile asks the engine side; and the preview keeps the viewport's aspect. Each
//! check also runs on an input it must reject (R-176, through `rejects`).

use std::sync::Arc;

use eframe::egui::{
    accesskit::Role, pos2, vec2, Event, Modifiers, PointerButton, Pos2, Rect, Vec2,
};
use engine::contract::render_state::{NodeId, Occupant, StainGraph};

use super::support::{click, frame_texts, headless, mock_app, nodes, rejects, FORMAT};
use crate::app::App;
use crate::capture::{render_frame, Shot, PIXELS_PER_POINT};
use crate::headless::Headless;
use crate::layout::Layout;
use crate::mock::canvas::MockCanvas;
use crate::mock::stain::{default_graph, look, COMBINER, FADE, RECOMPILED};
use crate::mock::MockEngine;
use crate::side::{EngineSide, MockSide};
use crate::stain::canvas::TITLE_H;
use crate::stain::preview::fit;
use crate::stain::Regions;
use engine::contract::view_ui::Mode;

/// The mock's colour's source and its brightness.
const OUTCOME: NodeId = NodeId(2);
const LATE_FADE: NodeId = NodeId(5);

/// `app` on the Stain page, two frames drawn.
fn on_stain(mut app: App<MockSide>) -> (App<MockSide>, Headless) {
    app.view.mode = Mode::Stain;
    let mut h = headless();
    for _ in 0..2 {
        let _ = h.frame(&mut app, Vec::new());
    }
    (app, h)
}

/// Clicks node `id`'s title on the canvas, selecting it.
fn select(app: &mut App<MockSide>, h: &mut Headless, id: NodeId) {
    let ppp = h.pixels_per_point();
    let origin = Regions::new(&Layout::new(h.screen(), ppp), ppp, false)
        .canvas
        .min;
    let g = app.snapshot().render.stain_graph.clone();
    let r = app
        .stain
        .canvas
        .drawn(&g, origin)
        .into_iter()
        .find(|d| d.id == id)
        .expect("the node is drawn")
        .rect;
    for events in crate::capture::click(pos2(r.min.x + 30.0, r.min.y + TITLE_H / 2.0)) {
        let _ = h.frame(app, events);
    }
    assert_eq!(app.stain.selected(), Some(id), "the node selected");
}

fn graph(app: &App<MockSide>) -> &StainGraph {
    &app.snapshot().render.stain_graph
}

fn depth(app: &App<MockSide>) -> u32 {
    app.snapshot().history.undo_depth
}

/// The console's messages from row `from` on.
fn logged(app: &App<MockSide>, from: usize) -> Vec<String> {
    app.console()[from..]
        .iter()
        .map(|e| e.message.clone())
        .collect()
}

/// An edit's three marks: the graph changed as `changed` says, one more step of undo, and its `SetField` logged.
fn check_one_edit(
    before: (&StainGraph, u32, usize),
    app: &App<MockSide>,
    changed: impl Fn(&StainGraph) -> bool,
) {
    let (g0, d0, rows) = before;
    assert!(changed(graph(app)), "the edit shows in the snapshot");
    assert_ne!(graph(app), g0);
    assert_eq!(depth(app), d0 + 1, "one step of undo");
    let sets: Vec<String> = logged(app, rows)
        .into_iter()
        .filter(|m| m.starts_with("SetField StainGraph"))
        .collect();
    assert_eq!(sets.len(), 1, "one SetField logged: {sets:?}");
}

#[test]
fn mock_stain_preview_a_choice_is_one_undoable_set_field() {
    let (mut app, mut h) = on_stain(mock_app());
    select(&mut app, &mut h, NodeId(0));
    let (g0, d0, rows) = (graph(&app).clone(), depth(&app), app.console().len());
    click(&mut h, &mut app, "Replace-L");
    let _ = h.frame(&mut app, Vec::new());
    let replaced = |g: &StainGraph| {
        g.node(COMBINER).map(|n| &n.occupant) == Some(&Occupant::Builtin("replace_l".into()))
    };
    check_one_edit((&g0, d0, rows), &app, replaced);
    // The look and the assembled code follow the graph.
    assert_eq!(look(graph(&app))[2], 1.0);
    let code = app.side().stain_content().expect("the mock's content").code;
    assert!(code.contains("return with_l(c, b);"), "{code}");
    // Undo restores the graph, and so the look and the code.
    app.undo();
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(graph(&app), &g0);
    assert_eq!(look(graph(&app)), look(&g0));
    assert!(app
        .side()
        .stain_content()
        .unwrap()
        .code
        .contains("return c * b;"));
    rejects("an edit with no undo step", || {
        check_one_edit((&g0, depth(&app), rows), &app, replaced)
    });
}

#[test]
fn mock_stain_preview_a_source_field_shifts_the_look() {
    let (mut app, mut h) = on_stain(mock_app());
    select(&mut app, &mut h, OUTCOME);
    let (g0, d0, rows) = (graph(&app).clone(), depth(&app), app.console().len());
    click(&mut h, &mut app, "ftle");
    let _ = h.frame(&mut app, Vec::new());
    check_one_edit((&g0, d0, rows), &app, |g| {
        g.node(OUTCOME).map(|n| &n.occupant) == Some(&Occupant::Field("ftle".into()))
    });
    assert_ne!(
        look(graph(&app))[3],
        look(&g0)[3],
        "the phase follows the field"
    );
    assert!(app
        .side()
        .stain_content()
        .unwrap()
        .code
        .contains("colour(ctx.ftle)"));
    rejects("the field unchanged", || {
        check_one_edit((&g0, d0, rows), &app, |g| {
            g.node(OUTCOME).map(|n| &n.occupant) == Some(&Occupant::Field("state".into()))
        })
    });
}

/// The look's every part, from the graph.
#[test]
fn mock_stain_preview_look_reads_the_graph() {
    let g = default_graph();
    assert_eq!(look(&g), [1.0, 0.12, 0.0, 0.0]);
    let mut pass = g.clone();
    pass.set_occupant(COMBINER, Occupant::Builtin("pass_through".into()))
        .unwrap();
    assert_eq!(look(&pass)[2], 2.0);
    let mut faded = g.clone();
    faded.set_param(LATE_FADE, FADE, vec![0.5]).unwrap();
    assert_eq!(look(&faded)[1], 0.5);
    // No colour wired: a grey ramp; no brightness: full.
    let mut bare = g.clone();
    bare.remove(NodeId(4)).unwrap();
    bare.remove(LATE_FADE).unwrap();
    assert_eq!(look(&bare)[..2], [0.0, 1.0]);
    let mut fate = g.clone();
    fate.set_occupant(OUTCOME, Occupant::Field("t_end_step".into()))
        .unwrap();
    assert_eq!(look(&fate)[3], 1.0 / 5.0);
    // A colour whose source reads no field of the list: phase 0.
    let mut none = g.clone();
    none.set_occupant(OUTCOME, Occupant::None).unwrap();
    assert_eq!(look(&none)[3], 0.0);
    assert_eq!(look(&none)[0], 1.0);
    // The assembled code reads each combiner input's field.
    let code = crate::mock::stain::assembled(&g);
    assert!(
        code.contains("return combine(colour(ctx.state), brightness(ctx.t_end_step));"),
        "{code}"
    );
    // A colour with no occupant, though wired, or with no field wired into it, is not live: a grey ramp.
    let mut empty = g.clone();
    empty.set_occupant(NodeId(4), Occupant::None).unwrap();
    assert_eq!(look(&empty)[0], 0.0);
    let mut unfed = g.clone();
    unfed.remove(OUTCOME).unwrap();
    assert_eq!(look(&unfed)[0], 0.0);
    rejects("a look blind to the fade", || {
        assert_eq!(look(&faded), look(&g))
    });
}

// --- Drawn ---------------------------------------------------------------------------------------------------------

fn gpu_app() -> (App<MockSide>, Headless, Arc<MockCanvas>) {
    let canvas = Arc::new(MockCanvas::new().expect("a GPU adapter for the mock's canvas"));
    let app = App::new(
        MockSide::new(MockEngine::frozen(), Some(canvas.clone())),
        FORMAT,
    );
    let (app, h) = on_stain(app);
    (app, h, canvas)
}

/// The preview's pixels, from `shot`, in `rect` (points) shrunk by a pixel.
fn preview_pixels(shot: &Shot, rect: Rect) -> Vec<[u8; 3]> {
    let px = |v: f32| (v * PIXELS_PER_POINT).round() as usize;
    let mut out = Vec::new();
    for y in px(rect.min.y) + 1..px(rect.max.y) - 1 {
        for x in px(rect.min.x) + 1..px(rect.max.x) - 1 {
            let i = (y * shot.size[0] as usize + x) * 4;
            out.push([shot.rgba[i], shot.rgba[i + 1], shot.rgba[i + 2]]);
        }
    }
    out
}

/// How many of `a`'s pixels differ from `b`'s.
fn differing(a: &[[u8; 3]], b: &[[u8; 3]]) -> usize {
    a.iter().zip(b).filter(|(p, q)| p != q).count()
}

/// The preview is drawn, in the look of the graph as it stands: an edit redraws it, and its undo restores it.
#[test]
fn mock_stain_preview_an_edit_redraws_the_preview() {
    let (mut app, mut h, canvas) = gpu_app();
    let shot = render_frame(&canvas, &mut app, &mut h);
    let rect = app.stain.preview.expect("the preview drawn");
    assert_eq!(
        canvas.look(),
        look(&default_graph()),
        "the preview's look is the graph's"
    );
    let before = preview_pixels(&shot, rect);
    let distinct: std::collections::BTreeSet<_> = before.iter().collect();
    assert!(
        distinct.len() > 16,
        "the preview is a picture: {} colours",
        distinct.len()
    );
    // An edit: the combiner to Pass-through, the colour unshaded by the fade.
    let edit = crate::stain::edited(graph(&app), |g| {
        g.set_occupant(COMBINER, Occupant::Builtin("pass_through".into()))
    })
    .expect("an edit");
    app.set_field(edit);
    let _ = h.frame(&mut app, Vec::new());
    let after = preview_pixels(&render_frame(&canvas, &mut app, &mut h), rect);
    assert_eq!(canvas.look()[2], 2.0);
    let changed = differing(&before, &after);
    assert!(
        changed > before.len() / 4,
        "{changed} of {} pixels changed",
        before.len()
    );
    app.undo();
    let _ = h.frame(&mut app, Vec::new());
    let undone = preview_pixels(&render_frame(&canvas, &mut app, &mut h), rect);
    assert_eq!(differing(&before, &undone), 0, "undo restores the preview");
    rejects("a preview unchanged", || {
        assert!(differing(&before, &undone) > before.len() / 4)
    });
}

/// The preview keeps the viewport's aspect: the Explore figure's, square for the slice (§G1).
#[test]
fn mock_stain_preview_keeps_the_viewport_aspect() {
    let (app, h) = on_stain(mock_app());
    let rect = app.stain.preview.expect("the preview drawn");
    let figure = Layout::new(h.screen(), h.pixels_per_point()).figure;
    let aspect = |r: Rect| r.width() / r.height();
    // Within a pixel, as each edge is.
    assert!(
        (rect.width() - rect.height() * aspect(figure)).abs() <= 1.0 / PIXELS_PER_POINT,
        "the preview {rect:?} keeps the figure's {figure:?}"
    );
    rejects("a stretched preview", || {
        let stretched = Rect::from_min_size(rect.min, vec2(rect.width() * 1.2, rect.height()));
        assert!((stretched.width() - stretched.height() * aspect(figure)).abs() <= 1.0)
    });
}

#[test]
fn mock_stain_preview_fit() {
    let area = |w: f32, h: f32| Rect::from_min_size(pos2(10.0, 20.0), vec2(w, h));
    // Wide area, square viewport: the height's square, centred across.
    assert_eq!(
        fit(area(300.0, 100.0), Vec2::splat(50.0)),
        Rect::from_min_size(pos2(110.0, 20.0), vec2(100.0, 100.0))
    );
    // Tall area: the width's square, at the top.
    assert_eq!(
        fit(area(100.0, 300.0), Vec2::splat(50.0)),
        Rect::from_min_size(pos2(10.0, 20.0), vec2(100.0, 100.0))
    );
    // The area's own aspect: the area itself.
    assert_eq!(fit(area(200.0, 100.0), vec2(4.0, 2.0)), area(200.0, 100.0));
    // A wide viewport in a square area.
    assert_eq!(
        fit(area(100.0, 100.0), vec2(2.0, 1.0)),
        Rect::from_min_size(pos2(10.0, 20.0), vec2(100.0, 50.0))
    );
    // An empty viewport, in either axis: a square.
    for empty in [vec2(0.0, 5.0), vec2(5.0, 0.0), Vec2::ZERO] {
        assert_eq!(
            fit(area(300.0, 100.0), empty),
            Rect::from_min_size(pos2(110.0, 20.0), vec2(100.0, 100.0))
        );
    }
    rejects("a stretched fit", || {
        assert_eq!(
            fit(area(300.0, 100.0), Vec2::splat(50.0)),
            area(300.0, 100.0)
        )
    });
}

// --- A slider's drag, and Recompile ----------------------------------------------------------------------------------

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

/// Dragging the fade's slider is live: the graph follows the slider every frame, the whole drag one history entry, one
/// `SetField` line and one recompile, at the release (R-96, R-458 F2).
#[test]
fn mock_stain_preview_a_slider_drag_is_live_and_one_entry() {
    let (mut app, mut h) = on_stain(mock_app());
    select(&mut app, &mut h, LATE_FADE);
    let slider = nodes(&mut h, &mut app)
        .into_iter()
        .find(|n| n.role == Role::Slider)
        .expect("the fade's slider");
    let r = slider.rect;
    let at = |f: f32| pos2(r.min.x + f * r.width(), r.center().y);
    let (g0, d0, rows) = (graph(&app).clone(), depth(&app), app.console().len());
    let fade = |g: &StainGraph| g.node(LATE_FADE).and_then(|n| n.params.get(FADE).cloned());
    let _ = h.frame(&mut app, vec![Event::PointerMoved(at(0.3))]);
    let _ = h.frame(&mut app, vec![button(at(0.3), true)]);
    let mut during = Vec::new();
    for f in [0.4, 0.5, 0.6, 0.7] {
        let _ = h.frame(&mut app, vec![Event::PointerMoved(at(f))]);
        during.push((f, depth(&app), fade(graph(&app)).map(|v| v[0])));
    }
    for (f, d, v) in &during {
        assert_eq!(*d, d0 + 1, "one entry while dragged: {during:?}");
        assert!(
            v.is_some_and(|v| (v - f64::from(*f)).abs() < 0.05),
            "the fade at {f}: {during:?}"
        );
    }
    rejects("a drag sent only at its end", || {
        assert!(during.iter().all(|(_, d, _)| *d == d0))
    });
    let recompiled = |app: &App<MockSide>| {
        logged(app, rows)
            .iter()
            .filter(|m| *m == RECOMPILED)
            .count()
    };
    assert_eq!(recompiled(&app), 0, "a recompile mid-drag");
    let _ = h.frame(&mut app, vec![button(at(0.7), false)]);
    let _ = h.frame(&mut app, Vec::new());
    check_one_edit((&g0, d0, rows), &app, |g| {
        fade(g).is_some_and(|v| (v[0] - 0.7).abs() < 0.05)
    });
    assert_eq!(recompiled(&app), 1, "one recompile at the release");
    rejects("a drag of an entry per move", || {
        assert_eq!(depth(&app), d0 + 4)
    });
}

/// Recompile asks the engine side, which logs it; auto-recompile asks after each edit, and not when it is off.
#[test]
fn mock_stain_preview_recompile() {
    let (mut app, mut h) = on_stain(mock_app());
    let recompiled = |app: &App<MockSide>, from: usize| {
        logged(app, from)
            .iter()
            .filter(|m| *m == RECOMPILED)
            .count()
    };
    let rows = app.console().len();
    click(&mut h, &mut app, "Recompile");
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(recompiled(&app, rows), 1);
    assert!(frame_texts(&mut h, &mut app)
        .iter()
        .any(|t| t == "Recompile"));
    // An edit, auto-recompile on: one more.
    select(&mut app, &mut h, NodeId(0));
    let rows = app.console().len();
    click(&mut h, &mut app, "Replace-L");
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(recompiled(&app, rows), 1);
    // Off: an edit recompiles nothing.
    click(&mut h, &mut app, "auto-recompile");
    assert!(!app.stain.auto_recompile);
    let rows = app.console().len();
    click(&mut h, &mut app, "Multiply");
    let _ = h.frame(&mut app, Vec::new());
    assert!(logged(&app, rows)
        .iter()
        .any(|m| m.starts_with("SetField StainGraph")));
    assert_eq!(recompiled(&app, rows), 0);
    rejects("a recompile with auto-recompile off", || {
        assert_eq!(recompiled(&app, rows), 1)
    });
}

/// A click on the fade's slider, no drag, is one `SetField` at the value clicked.
#[test]
fn mock_stain_preview_a_slider_click_is_one_set_field() {
    let (mut app, mut h) = on_stain(mock_app());
    select(&mut app, &mut h, LATE_FADE);
    let r = nodes(&mut h, &mut app)
        .into_iter()
        .find(|n| n.role == Role::Slider)
        .expect("the fade's slider")
        .rect;
    let at = pos2(r.min.x + 0.8 * r.width(), r.center().y);
    let (g0, d0, rows) = (graph(&app).clone(), depth(&app), app.console().len());
    for events in crate::capture::click(at) {
        let _ = h.frame(&mut app, events);
    }
    let _ = h.frame(&mut app, Vec::new());
    let fade = |g: &StainGraph| g.node(LATE_FADE).and_then(|n| n.params.get(FADE).cloned());
    check_one_edit((&g0, d0, rows), &app, |g| {
        fade(g).is_some_and(|v| (v[0] - 0.8).abs() < 0.05)
    });
    rejects("the fade unchanged", || {
        check_one_edit((&g0, d0, rows), &app, |g| fade(g).is_none())
    });
}

/// The preview's pipeline is built once per target format and reused.
#[test]
fn mock_stain_preview_pipeline_is_cached() {
    let canvas = MockCanvas::new().expect("a GPU adapter for the mock's canvas");
    let (a, _) = canvas.preview_pipeline(FORMAT);
    let (b, _) = canvas.preview_pipeline(FORMAT);
    assert!(a == b, "the pipeline rebuilt for the same format");
    let other = wgpu::TextureFormat::Rgba16Float;
    let (c, _) = canvas.preview_pipeline(other);
    assert!(a != c);
    let (d, _) = canvas.preview_pipeline(other);
    assert!(c == d);
    rejects("a rebuilt pipeline read as the cached one", || {
        assert!(a == c)
    });
}

/// The inspector's sections follow the node: a source's "Field", the combiner's "Mode" with its occupant selected,
/// no shader actions on OUT; and the Problems pane's heading counts the problems.
#[test]
fn mock_stain_preview_inspector_sections() {
    let (mut app, mut h) = on_stain(mock_app());
    select(&mut app, &mut h, OUTCOME);
    let texts = frame_texts(&mut h, &mut app);
    assert!(texts.iter().any(|t| t == "▾ Field") && !texts.iter().any(|t| t == "▾ Mode"));
    assert!(texts.iter().any(|t| t == "Edit this node's WGSL"));
    select(&mut app, &mut h, COMBINER);
    let texts = frame_texts(&mut h, &mut app);
    assert!(texts.iter().any(|t| t == "▾ Mode") && !texts.iter().any(|t| t == "▾ Field"));
    // The combiner's occupant, Multiply, is the choice shown selected.
    let all = nodes(&mut h, &mut app);
    let rect = |t: &str| all.iter().find(|n| n.text == t).map(|n| n.rect).unwrap();
    let fill = h.ctx.global_style().visuals.selection.bg_fill;
    let drawn = super::support::shapes(&mut h, &mut app);
    let lit = |r: Rect| {
        drawn.iter().any(|s| matches!(s, eframe::egui::Shape::Rect(x) if x.fill == fill && r.contains(x.rect.center())))
    };
    assert!(lit(rect("Multiply")) && !lit(rect("Replace-L")));
    app.stain.canvas.selection = [crate::mock::stain::OUT].into();
    let texts = frame_texts(&mut h, &mut app);
    assert!(texts.iter().any(|t| t == "OUT · to the style"), "{texts:?}");
    assert!(!texts.iter().any(|t| t == "Edit this node's WGSL"));
    assert!(texts.iter().any(|t| t == "Problems · 0"));
    let mut content = app.side().stain_content().unwrap();
    content.problems = vec!["a".into(), "b".into()];
    assert_eq!(
        crate::stain::problems::heading(Some(&content)),
        "Problems · 2"
    );
    assert_eq!(crate::stain::problems::heading(None), "Problems · 0");
    rejects("Replace-L shown selected", || {
        assert!(lit(rect("Replace-L")))
    });
}
