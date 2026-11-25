//! Saved stains: a node is saved with its code by value, and old saved stains keep opening and drawing the same.
//!
//! - `outcome.json`, `debug_view.json` and `mixed.json` are stains saved in the three old forms: `outcome` names a
//!   colour and a combiner by their names in their slots and reads a field (`builtin`, `field`); `debug_view` holds its
//!   zero source and colour as custom text (`custom`); `mixed` holds all three with params and a post. Each opens,
//!   draws the code it drew when it was saved ([`DREW`]), and is written in the new form, the bytes of
//!   `<name>.saved.json`, which open to the same graph and write themselves again.
//! - `before_change.json` is `mixed` saved in the new form when the shipped `combiner/multiply` was other code. It draws
//!   that code, as it did when saved, and is offered the update; applying it draws today's file.
//! - A saved node whose hash is not its text's is refused.
//!
//! `pinned.txt` holds each fixture's fragment and render keys and code digest; run with `PRINCIPIA_BLESS=1` to rewrite
//! it and the `.saved.json` files after a change meant to change them. [`DREW`] is not rewritten.

use std::fmt::Write as _;
use std::path::PathBuf;

use engine::contract::stain::assembled::node_source;
use engine::contract::stain::assembled::{assemble, Tier};
use engine::contract::stain::graph::{NodeId, NodeKind, Occupant, StainGraph};
#[cfg(feature = "controls")]
use engine::contract::stain::saved::{self, hash};
use validation::negative_control;

const FIXTURES: [&str; 3] = ["outcome", "debug_view", "mixed"];

/// The FNV-1a digest of the code each old fixture assembled to when it was saved, before the saved form changed.
const DREW: [(&str, &str); 3] = [
    ("outcome", "605fb40797da7873"),
    ("debug_view", "6c46d7a218439d84"),
    ("mixed", "f20a871f30e68fdd"),
];

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/saved_stains")
}

fn read(name: &str) -> String {
    std::fs::read_to_string(dir().join(format!("{name}.json"))).unwrap_or_else(|e| panic!("{e}"))
}

fn bless() -> bool {
    std::env::var_os("PRINCIPIA_BLESS").is_some()
}

fn open(name: &str, text: &str) -> StainGraph {
    serde_json::from_str(text).unwrap_or_else(|e| panic!("{name} does not open: {e}"))
}

fn save(g: &StainGraph) -> String {
    serde_json::to_string_pretty(g).unwrap_or_else(|e| panic!("{e}")) + "\n"
}

/// The FNV-1a digest of the graph's own assembled code, as sixteen hex digits.
fn code(name: &str, g: &StainGraph) -> String {
    let a = assemble(g, Tier::FULL).unwrap_or_else(|e| panic!("{name}: {e}"));
    format!("{:016x}", ledger::version::fnv1a64(a.code.as_bytes()))
}

/// Each old fixture, read by `text`: it draws what it drew, its new form is `<name>.saved.json`, and that form opens to
/// the same graph and writes itself. One line per fixture: its name, fragment key, render key and code digest.
fn pins(text: &dyn Fn(&str) -> String) -> String {
    let mut out = String::new();
    for (name, drew) in DREW {
        let g = open(name, &text(name));
        let digest = code(name, &g);
        assert!(
            digest == drew,
            "{name}: a saved stain draws differently: its code is {digest}, it drew {drew}"
        );
        let saved = save(&g);
        let path = dir().join(format!("{name}.saved.json"));
        if bless() {
            std::fs::write(&path, &saved).unwrap_or_else(|e| panic!("{e}"));
        }
        let want = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(saved, want, "{name} is written in other bytes");
        let again = open(name, &saved);
        assert_eq!(again, g, "{name}'s new form opens to another graph");
        assert_eq!(save(&again), saved, "{name}'s new form writes other bytes");
        let a = assemble(&g, Tier::FULL).unwrap_or_else(|e| panic!("{name}: {e}"));
        let render = g
            .canonical()
            .unwrap_or_else(|e| panic!("{name}: {e}"))
            .render_key();
        let _ = writeln!(out, "{name} {:016x} {render:016x} {digest}", a.key);
    }
    out
}

fn check_pins(got: &str) {
    let path = dir().join("pinned.txt");
    if bless() {
        std::fs::write(&path, got).unwrap_or_else(|e| panic!("{e}"));
    }
    let want = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(got, want, "a saved stain's keys changed");
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

/// Each old form, `builtin`, `field` and `custom`, is in a fixture, and its node comes back from the new form as the
/// form named it: the shipped source by its name, the field, the custom's text. `alter` edits the new form's text.
fn check_each_form(alter: &dyn Fn(String) -> String) {
    let mut seen = [false; 3];
    for name in FIXTURES {
        let old: serde_json::Value =
            serde_json::from_str(&read(name)).unwrap_or_else(|e| panic!("{e}"));
        let g = open(name, &read(name));
        let again = open(name, &alter(save(&g)));
        for (k, n) in old["nodes"].as_array().expect("nodes").iter().enumerate() {
            let o = &n["occupant"];
            let (form, want) = if let Some(b) = o.get("builtin") {
                (0, Occupant::Builtin(b.as_str().expect("a name").to_owned()))
            } else if let Some(f) = o.get("field") {
                (1, Occupant::Field(f.as_str().expect("a field").to_owned()))
            } else if let Some(c) = o.get("custom") {
                (2, Occupant::Custom(c.as_str().expect("text").to_owned()))
            } else {
                continue;
            };
            seen[form] = true;
            let id = NodeId(n["id"].as_u64().expect("an id") as u32);
            let back = &again.node(id).expect("the node").occupant;
            assert!(
                *back == want,
                "{name}'s node {k} comes back as {back:?}, not {want:?}"
            );
        }
    }
    assert_eq!(seen, [true; 3], "a form no fixture holds");
}

#[test]
fn each_old_form_round_trips() {
    check_each_form(&|s| s);
}

negative_control!(
    each_old_form_round_trips,
    "a shipped source's saved text one byte off, its hash made to match",
    expected = "comes back as",
    check_each_form(&|s| {
        let g: serde_json::Value = serde_json::from_str(&s).expect("JSON");
        let mut g = g;
        for n in g["nodes"].as_array_mut().expect("nodes") {
            if let Some(shader) = n["occupant"].get_mut("shader") {
                if shader["id"].as_str() == Some("combiner/pass_through") {
                    let text = shader["source"]
                        .as_str()
                        .expect("text")
                        .replace("return rgb;", "return rgb ;");
                    shader["hash"] = hash(&text).into();
                    shader["source"] = text.into();
                }
            }
        }
        serde_json::to_string(&g).expect("JSON")
    })
);

/// `mixed`'s combiner, node 0, the shipped `combiner/multiply`.
const COMBINER: NodeId = NodeId(0);

/// `before_change.json`, read by `load`: it draws the code it was saved with, not today's file; it is offered the
/// update to today's file, the code changed; applied, it draws today's `mixed` and is offered nothing.
fn check_before_change(load: &dyn Fn() -> StainGraph) {
    let g = load();
    let Some(Occupant::Pinned(p)) = g.node(COMBINER).map(|n| &n.occupant) else {
        panic!("the saved combiner does not draw as it was saved: it reads as today's file");
    };
    assert_eq!(p.id, "combiner/multiply");
    let mut as_custom = g.clone();
    as_custom
        .set_occupant(COMBINER, Occupant::Custom(p.source.clone()))
        .unwrap_or_else(|e| panic!("{e}"));
    let saved = assemble(&g, Tier::FULL).unwrap_or_else(|e| panic!("{e}"));
    let custom = assemble(&as_custom, Tier::FULL).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(saved.code, custom.code, "it does not draw its saved code");
    let today = open("mixed", &read_mixed());
    let now = assemble(&today, Tier::FULL).unwrap_or_else(|e| panic!("{e}"));
    // Not through the fixture: today's file with the saved body, as the combiner of today's stain.
    let file = node_source(NodeKind::Combiner, &Occupant::Builtin("multiply".into()))
        .unwrap_or_else(|e| panic!("{e}"))
        .expect("multiply's text");
    let body = file.replace("return rgb * b;", "return rgb * (0.5 + 0.5 * b);");
    assert_ne!(body, file, "multiply's body is not `rgb * b`");
    let mut then = today.clone();
    then.set_occupant(COMBINER, Occupant::Custom(body))
        .unwrap_or_else(|e| panic!("{e}"));
    let then = assemble(&then, Tier::FULL).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        saved.code, then.code,
        "it does not draw the code it was saved with"
    );
    assert_eq!(
        saved.key, then.key,
        "it is not keyed by the code it was saved with"
    );
    assert_ne!(saved.code, now.code, "the saved code is today's");
    assert_ne!(saved.key, now.key, "two codes share a key");
    let offers = g.updates();
    assert_eq!(offers.len(), 1, "{offers:?}");
    let u = &offers[0];
    assert_eq!((u.node, u.id.as_str()), (COMBINER, "combiner/multiply"));
    assert!(u.code_changed, "the code changed but the offer says not");
    assert_eq!(u.occupant, Occupant::Builtin("multiply".into()));
    let mut updated = g.clone();
    let done = updated.update(COMBINER).unwrap_or_else(|e| panic!("{e}"));
    assert!(done.reset.is_empty(), "{done:?}");
    assert_eq!(updated, today, "updated, it is not today's stain");
    assert!(
        updated.updates().is_empty(),
        "updated, it is still offered one"
    );
    assert!(updated.update(COMBINER).is_err(), "an update applied twice");
    // Saving keeps it pinned: the saved code survives a save and a reopen, in the fixture's own bytes.
    assert_eq!(
        save(&g),
        read("before_change"),
        "a pinned node saves other bytes"
    );
    assert_eq!(
        open("before_change", &save(&g)),
        g,
        "a pinned node does not survive a save"
    );
}

fn read_mixed() -> String {
    read("mixed")
}

fn before_change() -> StainGraph {
    open("before_change", &read("before_change"))
}

#[test]
fn a_stain_saved_before_a_shipped_source_changed_draws_as_saved_and_offers_the_update() {
    check_before_change(&before_change);
}

negative_control!(
    a_stain_saved_before_a_shipped_source_changed_draws_as_saved_and_offers_the_update,
    "a reader that resolves a saved node by its id",
    expected = "does not draw as it was saved",
    check_before_change(&|| saved::probe::by_id_in(before_change))
);

/// `before_change.json` with one byte of its saved combiner's text changed and its hash left: refused, naming why.
fn check_damaged(open_it: &dyn Fn(&str) -> Result<StainGraph, String>) {
    let text = read("before_change").replace("0.5 + 0.5 * b", "0.5 + 0.6 * b");
    assert_ne!(
        text,
        read("before_change"),
        "the fixture holds no such code"
    );
    match open_it(&text) {
        Ok(_) => panic!("a damaged saved node opened"),
        Err(e) => assert!(e.contains("is damaged"), "{e}"),
    }
}

fn from_str(text: &str) -> Result<StainGraph, String> {
    serde_json::from_str(text).map_err(|e| e.to_string())
}

#[test]
fn a_damaged_saved_node_is_refused() {
    check_damaged(&from_str);
}

negative_control!(
    a_damaged_saved_node_is_refused,
    "a reader that does not check the hash",
    expected = "a damaged saved node opened",
    check_damaged(&|t| saved::probe::hash_skipped_in(|| from_str(t)))
);

/// An old `builtin` naming no file of its slot is refused with that reason, and a version with no id is refused.
fn check_unreadable(name: &str) {
    let old = read("outcome").replace("\"outcome_state\"", &format!("\"{name}\""));
    let e = from_str(&old).expect_err("an old built-in naming no file opened");
    assert!(e.contains("cannot be read"), "{e}");
    let mut g: serde_json::Value =
        serde_json::from_str(&save(&open("debug_view", &read("debug_view")))).expect("JSON");
    let custom = g["nodes"]
        .as_array_mut()
        .expect("nodes")
        .iter_mut()
        .find_map(|n| {
            n["occupant"]
                .get_mut("shader")
                .filter(|s| s["id"].is_null())
        })
        .expect("a custom");
    custom["version"] = "2".into();
    let versioned = serde_json::to_string(&g).expect("JSON");
    let e = from_str(&versioned).expect_err("a version with no id opened");
    assert!(e.contains("no id"), "{e}");
}

#[test]
fn an_unreadable_old_form_is_refused_with_its_reason() {
    check_unreadable("outcome_nowhere");
}

negative_control!(
    an_unreadable_old_form_is_refused_with_its_reason,
    "a name a file does give the slot",
    expected = "an old built-in naming no file opened",
    check_unreadable("outcome_state")
);

/// A shipped node and its pasted text share the render key as they share the fragment key: `mixed`'s combiner, given
/// as `text`.
fn check_render_key(text: &dyn Fn(&str) -> String) {
    let g = open("mixed", &read("mixed"));
    let file = node_source(NodeKind::Combiner, &Occupant::Builtin("multiply".into()))
        .unwrap_or_else(|e| panic!("{e}"))
        .expect("multiply's text");
    let mut pasted = g.clone();
    pasted
        .set_occupant(COMBINER, Occupant::Custom(text(&file)))
        .unwrap_or_else(|e| panic!("{e}"));
    let keys = |g: &StainGraph| {
        let c = g.canonical().unwrap_or_else(|e| panic!("{e}"));
        (c.fragment_key(), c.render_key())
    };
    assert!(
        keys(&g) == keys(&pasted),
        "a shipped node and its pasted text have two render keys"
    );
}

#[test]
fn a_shipped_node_and_its_pasted_text_share_the_render_key() {
    check_render_key(&|t| t.to_owned());
}

negative_control!(
    a_shipped_node_and_its_pasted_text_share_the_render_key,
    "other code pasted",
    expected = "have two render keys",
    check_render_key(&|t| t.replace("return rgb * b;", "return rgb * b * b;"))
);
