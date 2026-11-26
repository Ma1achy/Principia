//! The canvas's code views as the capture shows them: Pipeline WGSL is the assembler's text for the graph and Node WGSL
//! the selected node's own code, each drawn as it is, character for character. Pass criteria, fixed before the first
//! run:
//! - The text a code view lays out is its text, byte for byte, tabs, trailing spaces and all; against a re-rendering of
//!   it, its tabs spread to spaces.
//! - The `pipeline_wgsl` step's capture shows the content's Pipeline WGSL, the assembler's for the mock's graph, and the
//!   `node_wgsl` step's after `click_node` the colour node's own code; against the capture with no step, which shows
//!   neither.

use std::sync::Arc;

use eframe::egui;
use engine::contract::stain::assembled::{assemble, node_source};

use super::support::rejects;
use crate::capture::{shoot_screen, Step};
use crate::mock::canvas::MockCanvas;
use crate::mock::stain::{content, default_graph};
use crate::stain::{code_view, View};

/// Asserts that `laid` is `text`, as it was given.
fn check_verbatim(laid: &str, text: &str) {
    assert_eq!(laid, text, "the code view re-rendered its text");
}

#[test]
fn a_code_view_lays_out_its_text_as_it_is() {
    let text = "fn shade(ctx: Ctx) -> vec3<f32> {\n\treturn rgb;   \n}\n// ünïcode · ✔\n\n";
    let ctx = egui::Context::default();
    let mut laid = String::new();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        laid = code_view::job(ui, text).text;
    });
    output.textures_delta.clear();
    check_verbatim(&laid, text);
    rejects("its tabs spread to spaces", || {
        check_verbatim(&laid.replace('\t', "    "), text)
    });
}

/// The names the capture of `02_stain` after `steps` shows.
fn shown(canvas: &Arc<MockCanvas>, steps: &[Step]) -> Vec<String> {
    let shot = shoot_screen(canvas.clone(), "02_stain", steps);
    shot.names.into_iter().map(|n| n.name).collect()
}

/// Asserts that `names` holds `text`.
fn check_shows(names: &[String], text: &str, what: &str) {
    assert!(names.iter().any(|n| n == text), "{what} not shown");
}

#[test]
fn the_pipeline_and_node_wgsl_scenarios_show_the_generated_code() {
    let canvas = Arc::new(MockCanvas::new().expect("a GPU adapter for the mock's canvas"));
    let g = default_graph();
    let pipeline = assemble(&g, crate::real::stain::TIER)
        .expect("the mock's graph assembles")
        .pipeline;
    assert_eq!(content(&g).pipeline, pipeline, "the mock's Pipeline WGSL");
    assert!(pipeline.contains("fn shade("), "{pipeline}");
    let colour = g
        .nodes()
        .find(|n| n.kind == engine::contract::render_state::NodeKind::Colour)
        .expect("a colour node");
    let own = node_source(colour.kind, &colour.occupant)
        .expect("its source")
        .expect("its code");
    let pipeline_view = shown(&canvas, &[Step::View(View::Pipeline)]);
    check_shows(&pipeline_view, &pipeline, "Pipeline WGSL");
    let node_view = shown(&canvas, &[Step::ClickNode, Step::View(View::Node)]);
    check_shows(&node_view, &own, "the colour node's code");
    let graph_view = shown(&canvas, &[Step::ClickNode]);
    rejects("the graph view", || {
        check_shows(&graph_view, &pipeline, "Pipeline WGSL")
    });
    rejects("the graph view", || {
        check_shows(&graph_view, &own, "the node's code")
    });
}
