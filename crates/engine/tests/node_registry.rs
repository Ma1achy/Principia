//! The library names what it ships: every row that loads a graph holds shipped occupants only, each named by its id
//! (or a field, or None), never pasted text; and each such graph assembles to the same code as the row placed as
//! text.

use engine::contract::stain::assembled::{assemble, library, node_source, Entry, Tier};
use engine::contract::stain::graph::{Occupant, StainGraph};
use engine::presets;
use validation::negative_control;

/// The rows that load a graph, each with it.
fn loading(entries: Vec<Entry>) -> Vec<(Entry, StainGraph)> {
    entries
        .into_iter()
        .filter_map(|e| {
            let g = e.graph.clone()?;
            Some((e, g))
        })
        .collect()
}

/// Each row's graph holds only None, fields and shipped sources named by an id the registry resolves.
fn check_shipped_only(rows: &[(Entry, StainGraph)]) {
    assert!(!rows.is_empty(), "no row loads a graph");
    for (e, g) in rows {
        for n in g.nodes() {
            match &n.occupant {
                Occupant::None | Occupant::Field(_) => {}
                Occupant::Builtin(id) => {
                    node_source(n.kind, &n.occupant).unwrap_or_else(|why| {
                        panic!(
                            "{}: node {} names `{id}`, which no file gives: {why}",
                            e.id, n.id.0
                        )
                    });
                }
                Occupant::Custom(_) => {
                    panic!(
                        "{}: node {} holds pasted text, not a shipped source",
                        e.id, n.id.0
                    )
                }
            }
        }
    }
}

#[test]
fn every_row_that_loads_holds_shipped_occupants_only() {
    let rows = loading(library().expect("the library"));
    eprintln!("{} library rows load a graph", rows.len());
    check_shipped_only(&rows);
}

negative_control!(
    every_row_that_loads_holds_shipped_occupants_only,
    "one row placed as text, as the library placed debug views before",
    expected = "holds pasted text",
    {
        let mut rows = loading(library().expect("the library"));
        let (e, g) = rows.iter_mut().find(|(e, _)| e.debug).expect("a debug row");
        let source = node_source(e.slot, &Occupant::Builtin(e.name.clone()))
            .expect("its source")
            .expect("not the identity");
        *g = presets::view(&source).expect("the row as text");
        check_shipped_only(&rows);
    }
);

/// `source` with every `//` comment, trailing white space and blank line dropped, as `declared_code` reads code: the
/// assembler's per-node label comment says how the node was placed, which is not code.
fn code(source: &str) -> String {
    let mut out = String::new();
    for line in source.lines() {
        let line = line.split_once("//").map_or(line, |(c, _)| c).trim_end();
        if !line.trim().is_empty() {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// Each row's graph, named by id, assembles to the code of the same row placed as text by `as_text`.
fn check_same_code(rows: &[(Entry, StainGraph)], as_text: &dyn Fn(&str) -> String) {
    let mut compared = 0;
    for (e, g) in rows {
        if !e.debug {
            continue;
        }
        let source = node_source(e.slot, &Occupant::Builtin(e.name.clone()))
            .expect("its source")
            .expect("not the identity");
        let text = presets::view(&as_text(&source)).expect("the row as text");
        let named = assemble(g, Tier::FULL).unwrap_or_else(|why| panic!("{}: {why}", e.id));
        let placed = assemble(&text, Tier::FULL).unwrap_or_else(|why| panic!("{}: {why}", e.id));
        assert_eq!(
            code(&named.code),
            code(&placed.code),
            "{}: named by id, it assembles to other code than its text",
            e.id
        );
        compared += 1;
    }
    eprintln!("{compared} debug rows: named and placed as text, the same code");
    assert!(compared > 0, "no debug row compared");
}

#[test]
fn every_row_named_assembles_to_the_code_of_its_text() {
    check_same_code(&loading(library().expect("the library")), &str::to_owned);
}

negative_control!(
    every_row_named_assembles_to_the_code_of_its_text,
    "text one statement longer than the file the id names",
    expected = "other code than its text",
    check_same_code(&loading(library().expect("the library")), &|t| format!(
        "{t}\nconst NAMED_CONTROL: f32 = 1.0;\n"
    ))
);
