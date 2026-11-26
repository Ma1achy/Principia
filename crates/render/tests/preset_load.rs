//! The preset library's files: each holds a preset's catalogue metadata and its whole graph, from one file.
//!
//! - `preset_load_files_are_the_directory`: the embedded list is exactly the `.json` files under `presets/`, each id
//!   once. Control: a list missing one file fails.
//! - `preset_load_metadata_and_graph`: every file reads; every debug preset is locked and in one of the six groups,
//!   every production one editable; each preset's graph names, as its colour node, the shipped source its id is, so the
//!   row and the graph cannot disagree. Control: a debug file marked editable fails.
//! - `preset_load_every_debug_view_has_a_file`: each debug colour entry of the registry has a file, and no file names
//!   an entry that does not exist. Control: a registry with an extra entry fails.

use std::collections::BTreeSet;
use std::path::Path;

use render::presets::library::{self, File, DEBUG_GROUPS};
use render::registry::{self, Category};
use validation::negative_control;

fn listed() -> BTreeSet<String> {
    library::FILES
        .iter()
        .map(|(p, _)| (*p).to_owned())
        .collect()
}

fn on_disk() -> BTreeSet<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("presets");
    let mut out = BTreeSet::new();
    for dir in ["production", "debug"] {
        for e in std::fs::read_dir(root.join(dir)).expect("a preset directory") {
            let p = e.expect("an entry").path();
            if p.extension().is_some_and(|x| x == "json") {
                out.insert(format!(
                    "{dir}/{}",
                    p.file_stem().and_then(|s| s.to_str()).expect("a name")
                ));
            }
        }
    }
    out
}

fn check_files_are_the_directory(listed: &BTreeSet<String>) {
    assert_eq!(
        *listed,
        on_disk(),
        "the embedded preset list is not the files under presets/"
    );
}

#[test]
fn preset_load_files_are_the_directory() {
    check_files_are_the_directory(&listed());
    let ids: BTreeSet<_> = library::files()
        .expect("files")
        .into_iter()
        .map(|f| f.id)
        .collect();
    assert_eq!(ids.len(), library::FILES.len(), "an id is used twice");
}

negative_control!(
    preset_load_files_are_the_directory,
    "a list missing one file",
    expected = "is not the files under presets",
    {
        let mut l = listed();
        let one = l.iter().next().cloned().expect("a file");
        l.remove(&one);
        check_files_are_the_directory(&l);
    }
);

/// The name of the node of `kind` in a file's graph.
fn node_name(f: &File, kind: &str) -> String {
    let nodes = f.graph["nodes"].as_array().expect("nodes");
    let n = nodes
        .iter()
        .find(|n| n["kind"] == kind)
        .unwrap_or_else(|| panic!("`{}` has no {kind} node", f.id));
    n["occupant"]["builtin"]
        .as_str()
        .unwrap_or_else(|| panic!("`{}`'s {kind} is not a shipped source by name", f.id))
        .to_owned()
}

fn check_metadata(f: &File) {
    assert_eq!(
        f.debug(),
        f.locked,
        "`{}`: debug is locked, production is not",
        f.id
    );
    if f.debug() {
        assert!(
            DEBUG_GROUPS.contains(&f.group.as_str()),
            "`{}` is in `{}`, no debug group",
            f.id,
            f.group
        );
    }
    let colour = node_name(f, "colour");
    let entry = registry::registry()
        .expect("the registry")
        .into_iter()
        .find(|e| e.id == f.id)
        .unwrap_or_else(|| panic!("`{}` is no registry entry", f.id));
    assert_eq!(
        colour,
        entry.name(),
        "`{}`'s graph shows `{colour}`, not its own entry",
        f.id
    );
}

#[test]
fn preset_load_metadata_and_graph() {
    for f in library::files().expect("files") {
        check_metadata(&f);
    }
}

negative_control!(
    preset_load_metadata_and_graph,
    "a debug file marked editable",
    expected = "debug is locked, production is not",
    {
        let mut f = library::files()
            .expect("files")
            .into_iter()
            .find(File::debug)
            .expect("a debug file");
        f.locked = false;
        check_metadata(&f);
    }
);

fn debug_colour_ids() -> BTreeSet<String> {
    registry::registry()
        .expect("the registry")
        .into_iter()
        .filter(|e| e.category == Category::Debug && e.slot == render::assemble::Kind::Colour)
        .map(|e| e.id)
        .collect()
}

fn check_every_debug_view(entries: &BTreeSet<String>) {
    let files: BTreeSet<String> = library::files()
        .expect("files")
        .into_iter()
        .filter(File::debug)
        .map(|f| f.id)
        .collect();
    assert_eq!(
        *entries, files,
        "the debug views and the debug preset files are not the same set"
    );
}

#[test]
fn preset_load_every_debug_view_has_a_file() {
    check_every_debug_view(&debug_colour_ids());
}

negative_control!(
    preset_load_every_debug_view_has_a_file,
    "a registry with an extra debug view",
    expected = "are not the same set",
    {
        let mut e = debug_colour_ids();
        e.insert("debug/generated/not_a_view".to_owned());
        check_every_debug_view(&e);
    }
);
