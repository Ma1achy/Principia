//! Every preset the engine builds, pinned by the code it assembles to: a preset that becomes a whole-graph payload
//! must draw bit for bit as before, so its canonical text and both keys may not move. The pin is a file, one line per
//! preset (`tests/data/preset_pins.txt`); set `BLESS_PRESET_PINS=1` to write it, which a change to a pin must justify.
//!
//! Canonical text, not `Occupant` equality: a custom holding the zero source and the shipped `zero` are two values and
//! one piece of code.

use engine::contract::stain::assembled::{assemble, Tier};
use engine::presets::{self, CoordinateMode};
use engine::stain::StainGraph;
use engine::structural::{self, Level};
use validation::negative_control;

fn line(name: &str, g: &StainGraph) -> String {
    let c = g.canonical().unwrap_or_else(|e| panic!("{name}: {e}"));
    let text = c.render_text();
    let code = assemble(g, Tier::FULL).unwrap_or_else(|e| panic!("{name}: {e}"));
    format!(
        "{name} fragment_key={:016x} render_key={:016x} text_hash={} code_hash={}",
        c.fragment_key(),
        c.render_key(),
        engine::contract::stain::saved::hash(&text),
        engine::contract::stain::saved::hash(&code.pipeline),
    )
}

/// Every preset today's code builds, by name.
fn graphs() -> Vec<(String, StainGraph)> {
    let mut out: Vec<(String, StainGraph)> = Vec::new();
    let mut push = |name: String, g: Result<StainGraph, engine::stain::GraphError>| {
        out.push((name.clone(), g.unwrap_or_else(|e| panic!("{name}: {e}"))));
    };
    push("outcome".into(), presets::outcome());
    push("uv_screen".into(), presets::uv_screen());
    push("uv_quad".into(), presets::uv_quad());
    push(
        "coordinate_uv".into(),
        presets::coordinate_view(CoordinateMode::Uv),
    );
    push(
        "coordinate_delta".into(),
        presets::coordinate_view(CoordinateMode::Delta),
    );
    for id in structural::VIEWS {
        push(format!("structural/{id}"), structural::view(id));
    }
    for (name, level) in [
        ("quad", Level::Quad),
        ("tile", Level::Tile),
        ("both", Level::Both),
    ] {
        push(format!("boundaries/{name}"), structural::boundaries(level));
    }
    for e in engine::contract::stain::assembled::library().unwrap_or_else(|e| panic!("{e}")) {
        if let Some(g) = e.graph {
            push(format!("library/{}", e.id), Ok(g));
        }
    }
    for p in engine::contract::stain::preset::library().unwrap_or_else(|e| panic!("{e}")) {
        out.push((format!("file/{}", p.id), p.graph));
    }
    out
}

fn pins() -> String {
    let mut s: String = graphs().iter().map(|(n, g)| line(n, g) + "\n").collect();
    s.insert_str(0, "# One line per preset: its keys and the hashes of its canonical text and its assembled code.\n");
    s
}

const FILE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/preset_pins.txt");

#[test]
fn preset_pins_hold() {
    let now = pins();
    if std::env::var_os("BLESS_PRESET_PINS").is_some() {
        std::fs::write(FILE, &now).expect("write the pins");
        return;
    }
    let stored = std::fs::read_to_string(FILE).expect("the pin file");
    for (a, b) in stored.lines().zip(now.lines()) {
        assert_eq!(a, b, "a preset's code moved");
    }
    assert_eq!(
        stored.lines().count(),
        now.lines().count(),
        "a preset was added or lost"
    );
}

negative_control!(
    preset_pins_hold,
    "a preset whose graph has one parameter changed no longer matches its pin",
    expected = "a preset's code moved",
    {
        let mut g = structural::boundaries(Level::Quad).expect("boundaries");
        let post = g.chain().into_iter().next().expect("the overlay post");
        g.set_param(post, "level", vec![2.0]).expect("param");
        let stored = std::fs::read_to_string(FILE).expect("the pin file");
        let pinned = stored
            .lines()
            .find(|l| l.starts_with("boundaries/quad "))
            .expect("the quad pin");
        assert_eq!(pinned, line("boundaries/quad", &g), "a preset's code moved");
    }
);
