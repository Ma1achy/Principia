//! Saved stains in the three saved forms, `builtin`, `field` and `custom`, open and draw the same. Each fixture in
//! `tests/data/saved_stains/` is a saved stain graph: `outcome.json` names a colour and a combiner by their in-slot ids
//! and reads a field; `debug_view.json` is a debug view with its zero source and its colour as custom text;
//! `mixed.json` holds all three forms with params and a post. Each opens,
//! saves back to the same bytes, and assembles to the code and keys `pinned.txt` holds. Run with `PRINCIPIA_BLESS=1`
//! to rewrite the pins after a change that is meant to change them.

use std::fmt::Write as _;
use std::path::PathBuf;

use engine::contract::stain::assembled::{assemble, Tier};
use engine::contract::stain::graph::StainGraph;
use validation::negative_control;

const FIXTURES: [&str; 3] = ["outcome", "debug_view", "mixed"];

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/saved_stains")
}

fn read(name: &str) -> String {
    std::fs::read_to_string(dir().join(format!("{name}.json"))).unwrap_or_else(|e| panic!("{e}"))
}

/// One line per fixture: its name, fragment key, render key and the FNV-1a digest of the graph's own assembled code.
fn pins(text: &dyn Fn(&str) -> String) -> String {
    let mut out = String::new();
    for name in FIXTURES {
        let saved = text(name);
        let g: StainGraph =
            serde_json::from_str(&saved).unwrap_or_else(|e| panic!("{name} does not open: {e}"));
        let again = serde_json::to_string_pretty(&g).unwrap_or_else(|e| panic!("{e}")) + "\n";
        assert_eq!(again, saved, "{name} saves back to other bytes");
        let a = assemble(&g, Tier::FULL).unwrap_or_else(|e| panic!("{name}: {e}"));
        let render = g
            .canonical()
            .unwrap_or_else(|e| panic!("{name}: {e}"))
            .render_key();
        let _ = writeln!(
            out,
            "{name} {:016x} {render:016x} {:016x}",
            a.key,
            ledger::version::fnv1a64(a.code.as_bytes())
        );
    }
    out
}

fn check_pins(got: &str) {
    let path = dir().join("pinned.txt");
    if std::env::var_os("PRINCIPIA_BLESS").is_some() {
        std::fs::write(&path, got).unwrap_or_else(|e| panic!("{e}"));
    }
    let want = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        got, want,
        "a saved stain draws differently: its code or keys changed"
    );
}

#[test]
fn old_saved_stains_open_and_draw_the_same() {
    check_pins(&pins(&read));
}

negative_control!(
    old_saved_stains_open_and_draw_the_same,
    "a saved combiner id read as another combiner draws differently",
    expected = "a saved stain draws differently",
    check_pins(&pins(
        &|name| read(name).replace("\"multiply\"", "\"replace_l\"")
    ))
);
