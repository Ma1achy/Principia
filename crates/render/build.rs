//! render's build script: the Print graph's registry is checked at build time (principia_print_graph.md §4.3, §11
//! test 8; REQ-RENDER-104). It registers every node entry in `src/print/nodes/` and checks every preset in
//! `src/print/presets/` against them, with the library's own check (`print::registry::check`), and fails the build,
//! naming the preset, the node and the parameter, where a preset names a node type that is not registered or sets a
//! value outside its range. Then it writes the files it checked into `$OUT_DIR/print_shipped.rs`, which the library
//! embeds (`print::NODES`, `print::WGSL`, `print::PRESETS`): a preset is data, so adding a medium needs no code when
//! its nodes exist.
//!
//! The fragment registry is scanned at build time too (gui_state_contract §3; REQ-GEN-020; R-815, RQ-677 (e)): the
//! script runs the library's own scan (`registry::scan::occupant_files`) on this crate and writes each occupant file
//! it finds into `$OUT_DIR/frag_registry.rs`, which the library embeds (`registry::SHIPPED`), so `registry()` reads
//! no file at run time. Adding a `.wgsl` file under a scanned directory reruns the script.

use std::fs;
use std::path::PathBuf;

// The registry's modules and the lexer it reads a pass with, compiled here as in the library, so the build runs the
// library's own check. What the check does not reach (the canonical serialisation, the codegen's inputs) is unused in
// this script.
#[allow(dead_code)]
#[path = "src/print/graph.rs"]
mod graph;
#[allow(dead_code)]
#[path = "src/lex.rs"]
mod lex;
#[allow(dead_code)]
#[path = "src/print/ports.rs"]
mod ports;
#[allow(dead_code)]
#[path = "src/print/registry.rs"]
mod registry;
// The fragment registry's scan, compiled here as in the library: `std` only, and every item of it used.
#[path = "src/registry/scan.rs"]
mod scan;

/// The node entries' folder: each entry's declaration, `<id>.json`, and its passes' WGSL files.
const NODES: &str = "src/print/nodes";
/// The presets' folder: each preset a graph, `<name>.json`.
const PRESETS: &str = "src/print/presets";

/// A file the build read: its name, its path and its text.
struct File {
    name: String,
    path: PathBuf,
    text: String,
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={NODES}");
    println!("cargo:rerun-if-changed={PRESETS}");
    for module in ["graph", "ports", "registry"] {
        println!("cargo:rerun-if-changed=src/print/{module}.rs");
    }
    let nodes = files(NODES, "json");
    let wgsl = files(NODES, "wgsl");
    let presets = files(PRESETS, "json");
    check_or_exit(&nodes, &wgsl, &presets);
    write_table(&nodes, &wgsl, &presets);
    write_frag_registry();
}

/// `$OUT_DIR/frag_registry.rs`: the occupant files of this crate, as the registry's scan finds them, each embedded.
fn write_frag_registry() {
    println!("cargo:rerun-if-changed=src/registry/scan.rs");
    for (root, _) in scan::ROOTS {
        println!("cargo:rerun-if-changed={root}");
    }
    let render = std::env::var_os("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
    let found = scan::occupant_files(&PathBuf::from(render))
        .unwrap_or_else(|e| panic!("cannot read {}: {}", e.path.display(), e.why));
    let dir = std::env::var_os("OUT_DIR").expect("cargo sets OUT_DIR for a build script");
    let file = PathBuf::from(dir).join("frag_registry.rs");
    fs::write(&file, scan::shipped_table(&found))
        .unwrap_or_else(|e| panic!("cannot write {}: {e}", file.display()));
}

/// The files of `dir` ending `.ext`, sorted by name.
fn files(dir: &str, ext: &str) -> Vec<File> {
    let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("cannot read {dir}: {e}"));
    let mut out = Vec::new();
    for entry in entries {
        let path = entry
            .unwrap_or_else(|e| panic!("cannot read {dir}: {e}"))
            .path();
        if path.extension().and_then(|x| x.to_str()) != Some(ext) {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_else(|| panic!("{} has no UTF-8 name", path.display()))
            .to_owned();
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        let path = path
            .canonicalize()
            .unwrap_or_else(|e| panic!("cannot resolve {}: {e}", path.display()));
        out.push(File { name, path, text });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// The check: every entry registers and every preset checks against the registry; the build fails otherwise, with
/// the check's message.
fn check_or_exit(nodes: &[File], wgsl: &[File], presets: &[File]) {
    if let Err(e) = registry::check(&pairs(nodes), &pairs(wgsl), &pairs(presets)) {
        eprintln!(
            "error: the Print graph's registry check failed (principia_print_graph.md §4.3, §11 test 8): {e}"
        );
        std::process::exit(1);
    }
}

/// Each file's name and text.
fn pairs(files: &[File]) -> Vec<(&str, &str)> {
    files
        .iter()
        .map(|f| (f.name.as_str(), f.text.as_str()))
        .collect()
}

/// `$OUT_DIR/print_shipped.rs`: each folder's files as `(name, text)`, the text embedded from the file.
fn write_table(nodes: &[File], wgsl: &[File], presets: &[File]) {
    let mut out = String::new();
    for (name, what, files) in [
        ("NODES", "The node entries' declarations", nodes),
        ("WGSL", "The node entries' passes", wgsl),
        ("PRESETS", "The presets", presets),
    ] {
        out.push_str(&format!(
            "/// {what}: each file's name and text.\npub const {name}: &[(&str, &str)] = &[\n"
        ));
        for f in files {
            let path = f
                .path
                .to_str()
                .unwrap_or_else(|| panic!("{} has no UTF-8 path", f.path.display()));
            out.push_str(&format!("    ({:?}, include_str!({path:?})),\n", f.name));
        }
        out.push_str("];\n");
    }
    let dir = std::env::var_os("OUT_DIR").expect("cargo sets OUT_DIR for a build script");
    let file = PathBuf::from(dir).join("print_shipped.rs");
    fs::write(&file, out).unwrap_or_else(|e| panic!("cannot write {}: {e}", file.display()));
}
