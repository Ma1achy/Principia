//! The editor draws a node from its description alone. The pass criteria, fixed before the first run:
//! - Every shipped source and every field, as the app places it and pasted as text, draws the same shapes on the canvas
//!   and in the inspector, and has the same scopes; none draws a `{ }`.
//! - A node of 40 parameters has 40 controls, each drawn inside the inspector when the keys reach it.
//! - Every control of the fixed set is drawn, every widget the inspector draws is one a key reaches, and each does what
//!   it edits by the keyboard and by the mouse.
//! - A swatches glyph and the inspector's chips are the node's own colours, of the set it shows.
//! - After a label-only edit the text and the labels shown are the edited source's, with nothing assembled.
//! - The library row of a graph is lit whether the graph names its shaders or holds their texts.
//! - A setting on a shipped shader leaves the graph naming it, and its fragment key as it was.
//! - The editor's code names no occupant.
//!
//! Each check also runs on an input it must reject (through `rejects`).

use std::collections::BTreeSet;

use eframe::egui::{self, Key, Modifiers, Rect, Shape};
use engine::contract::render_state::{Node, NodeId, NodeKind, Occupant, StainGraph};
use engine::contract::set_field::{Edit, RenderField, SetField};
use engine::contract::stain::assembled::{library, node_source, source_fields};
use engine::contract::stain::describe::{Control, Shown};
use engine::contract::view_ui::{Focus, KeyMode, Mode};

use super::mock_stain_preview::{graph, on_stain};
use super::reachability::{check_reached, drawn_controls, walk};
use super::support::{
    click_scope, focus, headless, keys_on, mock_app, nodes, press, rejects, FORMAT,
};
use crate::app::App;
use crate::headless::Headless;
use crate::keyboard::scopes::{Scope, ScopeTree};
use crate::keyboard::Keyboard;
use crate::layout::Layout;
use crate::mock::stain::content;
use crate::real::start;
use crate::side::{EngineSide, MockSide, RealSide};
use crate::stain::{
    self, catalogue, controls, library as drawer, Content, Page, Regions, Scratch, INSPECTOR,
};
use crate::windows::Keys;

/// A node using every control (`crates/render/tests/node_declarations.rs` holds it to that).
pub(super) const EXAMPLE: &str = include_str!("../../../render/tests/data/every_control.wgsl");

/// The node the tests look at, in a graph [`holding`] one.
const COMBINER: NodeId = NodeId(0);

/// A graph of the backbone and one node of `kind` with `occupant`, and that node.
pub(super) fn holding(kind: NodeKind, occupant: Occupant) -> (StainGraph, NodeId) {
    let mut g = StainGraph::new();
    let id = match kind {
        NodeKind::Combiner => {
            g.set_occupant(COMBINER, occupant).expect("a combiner");
            COMBINER
        }
        kind => g.add(kind, occupant).expect("a node"),
    };
    (g, id)
}

/// The Stain page, drawn by itself on a context kept between drawings.
pub(super) struct Bench {
    pub(super) h: Headless,
    time: f64,
}

/// What [`Bench::draw`] drew of a node: the canvas's and the inspector's shapes, the inspector's scopes, and where
/// each scope was drawn.
pub(super) struct Drawing {
    pub(super) shapes: Vec<Shape>,
    scopes: Vec<Scope>,
    pub(super) keyboard: Keyboard,
    pub(super) inspector: Rect,
}

impl Bench {
    pub(super) fn new() -> Self {
        Self {
            h: headless(),
            time: 0.0,
        }
    }

    /// Draws `g` with `content`, node `id` selected and the focus on `at`, three frames, the last kept: egui's layout
    /// and its animations have settled by then.
    pub(super) fn draw(
        &mut self,
        g: &StainGraph,
        content: &Content,
        id: NodeId,
        at: &[String],
    ) -> Drawing {
        let ppp = self.h.pixels_per_point();
        let layout = Layout::new(self.h.screen(), ppp);
        let regions = Regions::new(&layout, ppp, false);
        let mut s = Scratch::default();
        s.canvas.selection = [id].into();
        s.canvas.place(g);
        let mut tree = ScopeTree::new();
        stain::register(&mut tree);
        stain::join(&mut tree, g, &s, Some(content));
        let mut keyboard = Keyboard::new();
        let mut shapes = Vec::new();
        for _ in 0..3 {
            keyboard.clear_places();
            let mut input = egui::RawInput {
                screen_rect: Some(self.h.screen()),
                time: Some(self.time),
                ..Default::default()
            };
            input
                .viewports
                .entry(egui::ViewportId::ROOT)
                .or_default()
                .native_pixels_per_point = Some(ppp);
            self.time += 1.0;
            let page = Page {
                graph: g,
                content: Some(content),
                preview: None,
                viewport: layout.figure,
                focus: at,
            };
            let mut output = self.h.ctx.run_ui(input, |ui| {
                let mut keys = Keys {
                    keyboard: &mut keyboard,
                    focused: at.last().cloned(),
                };
                let (mut clicked, mut out) = (Vec::new(), stain::Out::default());
                stain::show(
                    ui,
                    &layout,
                    &page,
                    &mut s,
                    &mut keys,
                    &mut clicked,
                    &mut out,
                );
                assert!(clicked.is_empty() && out == stain::Out::default());
            });
            self.h
                .textures
                .append(std::mem::take(&mut output.textures_delta));
            shapes = output
                .shapes
                .into_iter()
                .filter(|c| {
                    regions.canvas.contains_rect(c.clip_rect)
                        || regions.right.contains_rect(c.clip_rect)
                        || regions.canvas.contains_rect(c.shape.visual_bounding_rect())
                        || regions.right.contains_rect(c.shape.visual_bounding_rect())
                })
                .map(|c| c.shape)
                .collect();
        }
        let scopes = all_scopes(&tree, INSPECTOR);
        Drawing {
            shapes,
            scopes,
            keyboard,
            inspector: regions.right,
        }
    }
}

/// Every scope below `parent`.
fn all_scopes(tree: &ScopeTree, parent: &str) -> Vec<Scope> {
    let mut out = Vec::new();
    for id in tree.children(Some(parent)) {
        out.extend(tree.get(id).copied());
        out.extend(all_scopes(tree, id));
    }
    out
}

/// The texts of `shapes`.
fn texts(shapes: &[Shape]) -> Vec<String> {
    fn walk(s: &Shape, out: &mut Vec<String>) {
        match s {
            Shape::Text(t) => out.push(t.galley.text().to_owned()),
            Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    shapes.iter().for_each(|s| walk(s, &mut out));
    out
}

// ── A built-in and its source pasted ───────────────────────────────────────────────────────────────────────────────

/// Every shipped source and every field, each its kind and its occupant.
pub(super) fn shipped() -> Vec<(NodeKind, Occupant)> {
    let fields = source_fields().expect("the fields").into_iter();
    library()
        .expect("the library")
        .into_iter()
        .map(|e| (e.slot, Occupant::Builtin(e.name)))
        .chain(fields.map(|f| (NodeKind::Source, Occupant::Field(f))))
        .collect()
}

/// Asserts that every one of `all` draws as its source pasted draws, each graph's content made by `content`; returns
/// how many were drawn.
fn check_pasted(all: &[(NodeKind, Occupant)], content: &dyn Fn(&StainGraph) -> Content) -> usize {
    let mut bench = Bench::new();
    for (kind, occupant) in all {
        let text = node_source(*kind, occupant)
            .expect("a source")
            .expect("not the identity");
        let mut draw = |occupant: &Occupant| {
            let (mut g, id) = holding(*kind, occupant.clone());
            // Every range's top, so the values drawn are the node's own.
            for u in g.uniforms(id).expect("uniforms") {
                if let Some((_, hi)) = u.range {
                    g.set_param(id, &u.name, vec![hi]).expect("in range");
                }
            }
            bench.draw(&g, &content(&g), id, &[])
        };
        let (ours, pasted) = (draw(occupant), draw(&Occupant::Custom(text)));
        assert!(ours.shapes.len() > 20, "{occupant:?}: little drawn");
        assert!(
            ours.shapes == pasted.shapes,
            "{occupant:?}: the shapes differ from its source's pasted: {:?} and {:?}",
            texts(&ours.shapes),
            texts(&pasted.shapes)
        );
        assert_eq!(ours.scopes, pasted.scopes, "{occupant:?}");
        let marks = texts(&pasted.shapes);
        assert!(!marks.iter().any(|t| t == "{ }"), "{occupant:?}: a mark");
    }
    all.len()
}

#[test]
fn a_built_in_and_its_source_pasted_draw_the_same_shapes() {
    let all = shipped();
    let drawn = check_pasted(&all, &content);
    eprintln!("{drawn} shipped sources drawn as their source pasted");
    assert!(drawn > 120, "{drawn} sources");
    let some: Vec<_> = all.into_iter().take(4).collect();
    rejects("a title taken from the id", || {
        check_pasted(&some, &|g| {
            let mut c = content(g);
            for n in g.nodes() {
                if let (Occupant::Builtin(id), Some(d)) = (&n.occupant, c.described.get_mut(&n.id))
                {
                    d.name = id.clone();
                }
            }
            c
        });
    });
}

/// Asserts that each library row is the one lit for its graph, with its shaders named and with their texts pasted.
fn check_lit(lit: &dyn Fn(&StainGraph) -> Option<usize>) -> usize {
    let mut rows = 0;
    for (row, entry) in library().expect("the library").iter().enumerate() {
        let Some(graph) = &entry.graph else { continue };
        let mut pasted = graph.clone();
        for n in graph.nodes() {
            if let Some(text) = node_source(n.kind, &n.occupant).expect("a source") {
                pasted
                    .set_occupant(n.id, Occupant::Custom(text))
                    .expect("its text");
            }
        }
        assert_ne!(graph, &pasted, "{}", entry.id);
        let title = |k: Option<usize>| k.and_then(|k| drawer::row(&content(graph), k));
        for g in [graph, &pasted] {
            let got = lit(g);
            assert!(got.is_some(), "{}: no row lit", entry.id);
            assert_eq!(got, lit(graph), "{}: pasted lights another", entry.id);
            let names = [entry.description.name.clone(), catalogue::told_apart(entry)];
            let named = title(got).is_some_and(|t| names.iter().any(|n| n == t));
            assert!(named, "{row}: {:?}", title(got));
        }
        rows += 1;
    }
    rows
}

#[test]
fn a_library_row_is_lit_for_its_shaders_pasted_too() {
    let rows = check_lit(&|g| content(g).loaded);
    eprintln!("{rows} rows lit");
    assert!(rows > 50, "{rows} rows");
    rejects("the row found by the shaders' names", || {
        check_lit(&|g| {
            let named = g
                .nodes()
                .all(|n| !matches!(n.occupant, Occupant::Custom(_)));
            content(g).loaded.filter(|_| named)
        });
    });
}

// ── 40 parameters ──────────────────────────────────────────────────────────────────────────────────────────────────

const MANY: usize = 40;

/// A colour of [`MANY`] parameters, in groups and controls of several kinds.
fn many() -> String {
    let mut text = String::from("// @name Many\n");
    for k in 0..MANY {
        let line = match k % 4 {
            0 => format!("// @uniform p{k}: f32 = 0.5 [0, 1]\n// @control p{k} slider \"P{k}\""),
            1 => format!(
                "// @uniform p{k}: u32 = 0 [0, 1]\n// @control p{k} toggle \"P{k}\" in \"G\""
            ),
            2 => format!("// @uniform p{k}: f32 = 0.5\n// @control p{k} fields \"P{k}\" in \"H\""),
            _ => format!(
                "// @uniform p{k}: vec3<f32> = (0.1, 0.2, 0.3)\n// @control p{k} colour \"P{k}\""
            ),
        };
        text.push_str(&line);
        text.push('\n');
    }
    text.push_str("fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.inputs[0].x); }\n");
    text
}

/// Asserts that each of the [`MANY`] parameters of `c`'s node `id` is drawn inside the inspector once focused.
fn check_many(g: &StainGraph, c: &Content, id: NodeId) {
    let mut bench = Bench::new();
    for k in 0..MANY {
        let scope = controls::scope(k);
        let d = bench.draw(g, c, id, &[INSPECTOR.into(), scope.into()]);
        let place = d
            .keyboard
            .place_of(scope)
            .unwrap_or_else(|| panic!("parameter {k} is not drawn"));
        assert!(
            place.shown().is_positive() && d.inspector.contains_rect(place.shown()),
            "parameter {k} is drawn at {place:?}, off the inspector"
        );
        assert!(
            texts(&d.shapes).contains(&format!("P{k}")),
            "parameter {k}'s label is not drawn"
        );
        assert!(
            d.scopes.iter().any(|s| s.id == scope),
            "no key reaches parameter {k}"
        );
    }
}

#[test]
fn a_node_of_forty_parameters_shows_all_forty() {
    let (g, id) = holding(NodeKind::Colour, Occupant::Custom(many()));
    let c = content(&g);
    assert_eq!(c.described[&id].controls.len(), MANY);
    check_many(&g, &c, id);
    let mut eight = c.clone();
    if let Some(d) = eight.described.get_mut(&id) {
        d.controls.truncate(8);
    }
    rejects("the first eight alone", || check_many(&g, &eight, id));
}

// ── Every control ──────────────────────────────────────────────────────────────────────────────────────────────────

/// The mock app on the Stain page with the example's node selected.
fn on_example() -> (App<MockSide>, Headless, NodeId) {
    on_colour(EXAMPLE)
}

/// The mock app on the Stain page with a colour of `text` selected.
pub(super) fn on_colour(text: &str) -> (App<MockSide>, Headless, NodeId) {
    let (g, id) = holding(NodeKind::Colour, Occupant::Custom(text.to_owned()));
    let mut app = mock_app();
    app.set_field(SetField {
        edit: Edit::Render(RenderField::StainGraph(g)),
        no_history: false,
    });
    let (mut app, mut h) = on_stain(app);
    app.stain.canvas.selection = [id].into();
    for _ in 0..2 {
        let _ = h.frame(&mut app, Vec::new());
    }
    (app, h, id)
}

/// Moves the focus to scope `at` and settles.
pub(super) fn go(app: &mut App<MockSide>, h: &mut Headless, at: &str) {
    app.view.focus.path = app.keyboard.tree(Mode::Stain).path_to(at);
    app.view.focus.mode = KeyMode::Navigation;
    for _ in 0..4 {
        let _ = h.frame(app, Vec::new());
    }
}

#[test]
fn every_control_is_drawn_and_every_widget_is_reached() {
    let (mut app, mut h, id) = on_example();
    let d = content(graph(&app)).described[&id].clone();
    let words: BTreeSet<&str> = d.controls.iter().map(|c| c.control.word()).collect();
    let all = [
        "slider",
        "log_slider",
        "stepper",
        "toggle",
        "choice",
        "flags",
        "colour",
        "angle",
        "gradient",
        "fields",
        "hidden",
    ];
    assert_eq!(words, BTreeSet::from(all), "the example uses every control");
    let start = Focus {
        path: vec![INSPECTOR.into()],
        mode: KeyMode::Navigation,
    };
    let reached = walk(app.keyboard.tree(Mode::Stain), &start, INSPECTOR);
    for (k, c) in d.controls.iter().enumerate() {
        let hidden = c.control == Control::Hidden;
        assert_eq!(
            reached.iter().any(|r| r == controls::scope(k)),
            !hidden,
            "{}",
            c.label
        );
    }
    let ppp = h.pixels_per_point();
    let inspector = Regions::new(&Layout::new(h.screen(), ppp), ppp, false).right;
    let mut widgets = 0;
    for at in reached.iter().filter(|r| *r != INSPECTOR) {
        go(&mut app, &mut h, at);
        let drawn = drawn_controls(&nodes(&mut h, &mut app), inspector);
        widgets += drawn.len();
        let tree = app.keyboard.tree(Mode::Stain);
        let check = |reached: &[String]| {
            check_reached(at, tree, reached, |id| app.keyboard.place_of(id), &drawn);
        };
        check(&reached);
        let shown = app.keyboard.place_of(at).map(|p| p.shown());
        assert!(
            shown.is_some_and(|r| r.is_positive() && inspector.contains_rect(r)),
            "{at} focused is drawn at {shown:?}"
        );
        // A widget a focused number is, left out of the walk, is missed.
        if controls::find(&controls::sections(&d), at)
            .is_some_and(|p| matches!(p.act, controls::Act::Step(_)))
        {
            let short: Vec<String> = reached.iter().filter(|r| *r != at).cloned().collect();
            rejects("a number no key reaches", || check(&short));
        }
    }
    assert!(widgets > 200, "{widgets} widgets");
}

/// The example's node.
fn node(app: &App<MockSide>, id: NodeId) -> Node {
    graph(app).node(id).expect("the node").clone()
}

/// Uniform `name`'s value in node `id`.
pub(super) fn value(app: &App<MockSide>, id: NodeId, name: &str) -> Vec<f64> {
    let declared = graph(app).uniforms(id).expect("the uniforms");
    let default = declared.into_iter().find(|u| u.name == name);
    node(app, id)
        .params
        .get(name)
        .cloned()
        .unwrap_or_else(|| default.expect("a uniform").default)
}

/// Asserts each uniform's value in `got` is `want`'s.
pub(super) fn check_values(got: &[(&str, Vec<f64>)], want: &[(&str, Vec<f64>)]) {
    for ((name, a), (_, b)) in got.iter().zip(want) {
        let close = a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-9);
        assert!(close, "{name} is {a:?}, not {b:?}");
    }
    assert_eq!(got.len(), want.len());
}

/// Each control's step, or press, by the keyboard: its scope, the uniform and its value after one Enter and, for a
/// number, one →. The scopes are by the display order: mode, bands, scale, strength, invert, mask, warm, cool, hue,
/// the ramp, offset.
pub(super) fn steps() -> Vec<(&'static str, &'static str, Vec<f64>)> {
    let up = |v: f64| controls::linear(controls::code(v) + 1.0);
    vec![
        ("stain_insp_param_0_2", "mode", vec![2.0]),
        ("stain_insp_param_1", "bands", vec![5.0]),
        ("stain_insp_param_2", "scale", vec![1e4_f64.powf(0.01)]),
        ("stain_insp_param_3", "strength", vec![0.51]),
        ("stain_insp_param_4", "invert", vec![1.0]),
        ("stain_insp_param_5_1", "mask", vec![5.0]),
        ("stain_insp_param_6_1", "warm", vec![1.0, up(0.5), 0.2]),
        ("stain_insp_param_7_2", "cool", vec![0.2, 0.4, up(0.95)]),
        ("stain_insp_param_8", "hue", vec![1f64.to_radians()]),
        ("stain_insp_param_9_0_0", "low", vec![up(0.0), 0.0, 0.0]),
        ("stain_insp_param_9_1_1", "mid", vec![0.5, up(0.5), 0.5]),
        ("stain_insp_param_10_1", "offset", vec![0.0, 0.01]),
    ]
}

#[test]
fn every_control_edits_by_the_keyboard() {
    let (mut app, mut h, id) = on_example();
    // `cool`'s blue is 1.0, at its top: set below it, so a step shows.
    let edit = stain::edited(graph(&app), |g| {
        g.set_param(id, "cool", vec![0.2, 0.4, 0.95])
    })
    .expect("a value");
    app.set_field(edit);
    let _ = h.frame(&mut app, Vec::new());
    keys_on(&mut h, &mut app);
    let (mut got, mut want) = (Vec::new(), Vec::new());
    for (scope, uniform, expected) in steps() {
        go(&mut app, &mut h, scope);
        assert_eq!(focus(&app).last(), Some(&scope));
        let (g0, depth) = (graph(&app).clone(), app.snapshot().history.undo_depth);
        press(&mut h, &mut app, Key::Enter, Modifiers::NONE);
        if app.view.focus.mode == KeyMode::Interaction {
            press(&mut h, &mut app, Key::ArrowRight, Modifiers::NONE);
            press(&mut h, &mut app, Key::Enter, Modifiers::NONE);
        }
        let _ = h.frame(&mut app, Vec::new());
        assert_ne!(graph(&app), &g0, "{scope}: no edit");
        assert_eq!(
            app.snapshot().history.undo_depth,
            depth + 1,
            "{scope}: one SetField"
        );
        assert_eq!(
            node(&app, id).occupant,
            Occupant::Custom(EXAMPLE.to_owned()),
            "{scope}: a setting made a shader"
        );
        got.push((uniform, value(&app, id, uniform)));
        want.push((uniform, expected));
    }
    check_values(&got, &want);
    let mut wrong = want.clone();
    wrong[1].1 = vec![4.31];
    rejects("a whole number stepped by a share of its range", || {
        check_values(&got, &wrong)
    });
}

/// Asserts that `after` names the shaders `before` does, and is one code.
fn check_no_shader_made(before: &StainGraph, after: &StainGraph) {
    assert_ne!(before, after, "no edit");
    for (a, b) in before.nodes().zip(after.nodes()) {
        assert_eq!(a.occupant, b.occupant, "a setting made a shader");
    }
    assert_eq!(catalogue::key(before), catalogue::key(after));
}

/// A shipped shader's settings leave it shipped, and its code as it was.
#[test]
fn a_setting_on_a_shipped_shader_makes_no_shader() {
    let palette = Occupant::Builtin(engine::contract::legend::outcome_palette().to_owned());
    let (g, id) = holding(NodeKind::Colour, palette);
    let mut app = mock_app();
    app.set_field(SetField {
        edit: Edit::Render(RenderField::StainGraph(g.clone())),
        no_history: false,
    });
    let (mut app, mut h) = on_stain(app);
    app.stain.canvas.selection = [id].into();
    let _ = h.frame(&mut app, Vec::new());
    keys_on(&mut h, &mut app);
    let d = content(&g).described[&id].clone();
    let at = |uniform: &str| {
        let k = d.controls.iter().position(|c| c.uniform == uniform);
        controls::scope(k.expect("the control"))
    };
    for scope in [
        format!("{}_1", at("swatch_set")),
        format!("{}_0", at("bounded")),
    ] {
        let before = graph(&app).clone();
        go(&mut app, &mut h, stain::intern(&scope));
        press(&mut h, &mut app, Key::Enter, Modifiers::NONE);
        if app.view.focus.mode == KeyMode::Interaction {
            press(&mut h, &mut app, Key::ArrowRight, Modifiers::NONE);
            press(&mut h, &mut app, Key::Enter, Modifiers::NONE);
        }
        let _ = h.frame(&mut app, Vec::new());
        check_no_shader_made(&before, graph(&app));
        let text = node_source(NodeKind::Colour, &node(&app, id).occupant);
        let mut made = graph(&app).clone();
        made.set_occupant(id, Occupant::Custom(text.expect("a source").expect("text")))
            .expect("its text");
        rejects("a setting that writes the shader", || {
            check_no_shader_made(&before, &made)
        });
    }
}

/// The widgets that are no number, by the mouse: a toggle, a choice, a flag, a colour's chip, a fold.
#[test]
fn the_buttons_and_the_folds_act_by_the_mouse() {
    let (mut app, mut h, id) = on_example();
    for (scope, uniform, want) in [
        ("stain_insp_param_4", "invert", 1.0),
        ("stain_insp_param_0_1", "mode", 1.0),
        ("stain_insp_param_5_0", "mask", 6.0),
        ("stain_insp_param_5_0", "mask", 7.0),
    ] {
        go(&mut app, &mut h, scope);
        click_scope(&mut h, &mut app, scope);
        let _ = h.frame(&mut app, Vec::new());
        assert_eq!(value(&app, id, uniform), [want], "{scope}");
    }
    // The focus off the colours: none shows its channels until its chip is clicked, and again hides them.
    let chip = "stain_insp_param_6";
    let red = "stain_insp_param_6_0";
    go(&mut app, &mut h, chip);
    assert!(app.keyboard.place_of(red).is_none());
    click_scope(&mut h, &mut app, chip);
    let _ = h.frame(&mut app, Vec::new());
    assert!(app.keyboard.place_of(red).is_some(), "the channels");
    click_scope(&mut h, &mut app, chip);
    let _ = h.frame(&mut app, Vec::new());
    assert!(app.keyboard.place_of(red).is_none());
    // A fold hides its section's controls, from the keys too, and brings them back.
    let heading = "stain_insp_section_1";
    let strength = "stain_insp_param_3";
    let held = |app: &App<MockSide>| app.keyboard.tree(Mode::Stain).get(strength).is_some();
    go(&mut app, &mut h, heading);
    assert!(held(&app));
    click_scope(&mut h, &mut app, heading);
    let _ = h.frame(&mut app, Vec::new());
    assert!(!held(&app) && app.keyboard.place_of(strength).is_none());
    rejects("a folded control drawn", || assert!(held(&app)));
    go(&mut app, &mut h, heading);
    click_scope(&mut h, &mut app, heading);
    let _ = h.frame(&mut app, Vec::new());
    assert!(held(&app));
}

// ── The glyph ──────────────────────────────────────────────────────────────────────────────────────────────────────

/// The fills of the chips `shapes` draws in a row of `n` at one height on the canvas.
fn chips(d: &Drawing, n: usize) -> Vec<Vec<egui::Color32>> {
    let mut rows: Vec<(Rect, Vec<egui::Color32>)> = Vec::new();
    for s in &d.shapes {
        if let Shape::Rect(r) = s {
            match rows.last_mut() {
                Some((last, row))
                    if last.min.y == r.rect.min.y
                        && (last.max.x + 1.0 - r.rect.min.x).abs() < 0.01 =>
                {
                    *last = r.rect;
                    row.push(r.fill);
                }
                _ => rows.push((r.rect, vec![r.fill])),
            }
        }
    }
    rows.into_iter()
        .map(|(_, row)| row)
        .filter(|row| row.len() == n)
        .collect()
}

#[test]
fn the_swatches_are_the_nodes_own_and_of_the_set_it_shows() {
    let palette = Occupant::Builtin(engine::contract::legend::outcome_palette().to_owned());
    let (g, id) = holding(NodeKind::Colour, palette);
    let mut bench = Bench::new();
    let mut drawn = |g: &StainGraph| {
        let rows = chips(&bench.draw(g, &content(g), id, &[]), 9);
        assert_eq!(rows.len(), 1, "one row of nine chips");
        rows.into_iter().next().expect("the row")
    };
    let value = |g: &StainGraph, uniform: &str| {
        let d = &content(g).described[&id];
        let c = d.controls.iter().find(|c| c.uniform == uniform);
        let v = &c.expect("the control").value;
        controls::colour([v[0], v[1], v[2]])
    };
    let set = |g: &StainGraph, name: &str, v: Vec<f64>| {
        let mut g = g.clone();
        g.set_param(id, name, v).expect("a value");
        g
    };
    let default = drawn(&g);
    let of_default: Vec<_> = engine::contract::legend::SWATCHES
        .iter()
        .map(|s| egui::Color32::from_rgb(s.srgb8[0], s.srgb8[1], s.srgb8[2]))
        .collect();
    assert_eq!(default, of_default, "a chip is its 8-bit code");
    let edited = set(&g, "bounded", vec![1.0, 0.5, 0.0]);
    let check_first = |g: &StainGraph, row: &[egui::Color32], uniform: &str| {
        assert_eq!(row[0], value(g, uniform));
    };
    check_first(&edited, &drawn(&edited), "bounded");
    rejects("the default colours", || {
        check_first(&edited, &default, "bounded")
    });
    let other = set(&edited, "swatch_set", vec![1.0]);
    let other = set(&other, "oklab_bounded", vec![0.0, 0.25, 1.0]);
    check_first(&other, &drawn(&other), "oklab_bounded");
    rejects("the set not shown", || {
        check_first(&other, &drawn(&other), "bounded")
    });
}

#[test]
fn every_glyph_of_the_set_is_drawn() {
    let mut bench = Bench::new();
    for (line, want) in [
        ("// @glyph name", Shown::Name),
        ("// @glyph dropdown", Shown::Dropdown),
        ("// @glyph ramp", Shown::Ramp),
        ("// @glyph none", Shown::None),
        (
            "// @glyph gradient",
            Shown::Gradient(vec![[0.0; 3], [0.5; 3], [1.0; 3]]),
        ),
    ] {
        let text = EXAMPLE.replace("// @glyph swatches \"Tints\"", line);
        let (g, id) = holding(NodeKind::Colour, Occupant::Custom(text));
        let c = content(&g);
        assert_eq!(c.described[&id].glyph, want);
        let d = bench.draw(&g, &c, id, &[]);
        // The node's name: its title's galley reads "colour · Banded tint", the glyph's the name alone.
        let named = texts(&d.shapes)
            .iter()
            .filter(|t| *t == "Banded tint")
            .count();
        let faces = matches!(want, Shown::Name | Shown::Dropdown);
        assert_eq!(named, usize::from(faces), "{line}");
        let row = |n: usize| d.shapes.iter().filter(|s| is_chip(s)).count() >= n;
        assert_eq!(
            row(20),
            matches!(want, Shown::Ramp | Shown::Gradient(_)),
            "{line}"
        );
        rejects("another glyph", || {
            assert_eq!(named, usize::from(!faces), "{line}")
        });
    }
}

/// Whether `s` is a strip of a ramp or a gradient glyph: a rect as high as a glyph's row, narrower than a swatch.
fn is_chip(s: &Shape) -> bool {
    matches!(s, Shape::Rect(r)
        if (r.rect.height() - (crate::stain::canvas::GLYPH_H - 10.0)).abs() < 0.01
            && r.rect.width() < 8.0)
}

// ── A label-only edit ──────────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn after_a_label_edit_the_text_shown_is_the_occupants_own() {
    let with = |text: &str| {
        let mut g = start().1.stain_graph;
        g.set_occupant(NodeId(3), Occupant::Custom(text.to_owned()))
            .expect("a colour");
        g
    };
    let (sim, render) = start();
    let (mut app, mut h) = on_stain(App::new(RealSide::new(sim, render), FORMAT));
    let mut show = |text: &str| -> (Content, u64) {
        app.set_field(SetField {
            edit: Edit::Render(RenderField::StainGraph(with(text))),
            no_history: false,
        });
        app.side().recompile();
        app.side().wait_for_stain();
        let _ = h.frame(&mut app, Vec::new());
        let c = app.side().stain_content().expect("content");
        let n = app.side().assemblies();
        (c, n)
    };
    let (before, n0) = show(EXAMPLE);
    let edited = EXAMPLE
        .replace("// @name Banded tint", "// @name Tinted bands")
        .replace("\"Strength\"", "\"Amount\"");
    let (after, n1) = show(&edited);
    assert_eq!(n1, n0, "a label edit assembled");
    assert_eq!(after.code, before.code);
    let check = |c: &Content| {
        assert_eq!(c.node_code[&NodeId(3)], edited);
        let d = &c.described[&NodeId(3)];
        assert_eq!(d.name, "Tinted bands");
        assert!(d.controls.iter().any(|c| c.label == "Amount"));
    };
    check(&after);
    rejects("the content of the source compiled", || check(&before));
}

// ── No occupant in the editor's code ───────────────────────────────────────────────────────────────────────────────

/// The files that draw a node and act on it.
const CODE: [(&str, &str); 5] = [
    ("canvas.rs", include_str!("../stain/canvas.rs")),
    ("controls.rs", include_str!("../stain/controls.rs")),
    ("library.rs", include_str!("../stain/library.rs")),
    ("mod.rs", include_str!("../stain/mod.rs")),
    ("preview.rs", include_str!("../stain/preview.rs")),
];

/// Asserts that no file of `code` names an occupant: a kind of one, or a shipped source's id or name in its strings.
fn check_code(code: &[(&str, String)]) {
    let ids: Vec<String> = library()
        .expect("the library")
        .into_iter()
        // `field` is also the name of a port's type, which the canvas writes as a string; the source's whole id stays.
        .flat_map(|e| [e.id, e.name])
        .filter(|id| id != "field")
        .collect();
    for (file, text) in code {
        let lines = text.lines().filter(|l| !l.trim_start().starts_with("//"));
        for line in lines {
            assert!(
                !line.contains("Occupant"),
                "{file} names an occupant: {line}"
            );
            for id in &ids {
                let quoted = format!("\"{id}\"");
                assert!(!line.contains(&quoted), "{file} names `{id}`: {line}");
            }
        }
    }
}

#[test]
fn the_editors_code_names_no_occupant() {
    let code: Vec<(&str, String)> = CODE.iter().map(|(f, t)| (*f, (*t).to_owned())).collect();
    check_code(&code);
    for planted in [
        "    if matches!(n.occupant, Occupant::Custom(_)) {",
        "    if name == \"outcome_state\" {",
    ] {
        let mut branch = code.clone();
        branch[0].1.push_str(planted);
        rejects("a branch on which a node is", || check_code(&branch));
    }
}
