//! The Print graph tests' fixtures (`fixtures/print_graph/`; TASK-M7-54): the stand-in node entries and presets the
//! verifies of REQ-RENDER-101 to REQ-RENDER-104 name, none of them shipped. `tests/print_*.rs` include this file with
//! `#[path]`; a file that uses only some of the items allows the rest's dead code.

use std::collections::BTreeMap;
use std::path::PathBuf;

use render::print::graph::Graph;
use render::print::registry::Registry;

/// The fixture node entries' declarations.
pub const FIXTURE_NODES: [&str; 3] = ["linocut.json", "polaroid.json", "blur.json"];

/// The fixture node entries' passes.
pub const FIXTURE_WGSL: [&str; 4] = [
    "linocut.wgsl",
    "polaroid.wgsl",
    "blur_h.wgsl",
    "blur_v.wgsl",
];

/// The text of `fixtures/print_graph/<path>`.
pub fn fixture(path: &str) -> String {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/print_graph")
        .join(path);
    std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display()))
}

/// Each WGSL file, shipped or fixture, by name.
pub fn wgsl(file: &str) -> Option<String> {
    render::print::WGSL
        .iter()
        .find(|(name, _)| *name == file)
        .map(|(_, text)| (*text).to_owned())
        .or_else(|| {
            FIXTURE_WGSL
                .contains(&file)
                .then(|| fixture(&format!("nodes/{file}")))
        })
}

/// The shipped registry with the fixture entries registered beside it.
pub fn registry() -> Registry {
    let mut registry = render::print::shipped().registry.clone();
    for decl in FIXTURE_NODES {
        registry
            .register(&fixture(&format!("nodes/{decl}")), &wgsl)
            .unwrap_or_else(|e| panic!("the fixture {decl} does not register: {e}"));
    }
    registry
}

/// The shipped presets and the fixture `Polaroid`, by name.
pub fn presets() -> BTreeMap<String, Graph> {
    let mut presets = render::print::shipped().presets.clone();
    let polaroid = Graph::parse(&fixture("presets/Polaroid.json")).expect("the fixture Polaroid");
    presets.insert(polaroid.name.clone(), polaroid);
    presets
}
