//! The ledger's index bases (dd_generation_root §3.8; R-449; REQ-GEN-036), in the ledger's own tests: the key that
//! states the base of each body- or pair-indexed field's ids (`payload::INDEX_BASES`), the fields it covers
//! (`payload::INDEXED`), and the check that refuses generation of the export decoder for an indexed field whose base
//! the key does not state (`gen::export::index_refused`). The engine's `export_index_base_*` check the generated
//! decoder. Each test has a registered negative control (R-176).

use ledger::gen::export::{self, index_refused};
use ledger::gen::{self, generate_with_index_bases, GenError};
use ledger::payload::{Indexed, BODY_BASE, INDEXED, INDEX_BASES, PAIR_BASE};
use ledger::schema::Entry;
use validation::negative_control;

/// The payload ledger's entries.
fn entries() -> Vec<Entry> {
    gen::validate(&ledger::layout()).unwrap_or_else(|e| panic!("{e}"))
}

/// `INDEX_BASES` with `field`'s text replaced by `text`, or dropped where `text` is `None`.
fn bases_with(field: &str, text: Option<&'static str>) -> Vec<(&'static str, &'static str)> {
    INDEX_BASES
        .iter()
        .filter_map(|&(f, t)| match (f == field, text) {
            (true, Some(t)) => Some((f, t)),
            (true, None) => None,
            (false, _) => Some((f, t)),
        })
        .collect()
}

/// Checks the refusals `refused` gives: none for the ledger's own key; for an indexed field whose text is dropped or
/// does not state its base, one line naming the field and the base it lacks; none for an indexed field the layout does
/// not hold.
fn check_refusals(refused: impl Fn(&[Entry], &[(&str, &str)]) -> Vec<String>) {
    let entries = entries();
    assert_eq!(
        refused(&entries, &INDEX_BASES),
        Vec::<String>::new(),
        "the ledger's own key"
    );
    for (field, text, lacks) in [
        ("m1", None, BODY_BASE),
        (
            "r",
            Some("components 2j and 2j + 1 are body j's"),
            BODY_BASE,
        ),
        ("dmin_pair", Some(BODY_BASE), PAIR_BASE),
        ("detail", Some(BODY_BASE), PAIR_BASE),
        ("detail", Some(PAIR_BASE), BODY_BASE),
        ("detail", None, BODY_BASE),
    ] {
        let lines = refused(&entries, &bases_with(field, text));
        assert_eq!(lines.len(), 1, "`{field}` with {text:?}: {lines:?}");
        assert!(
            lines[0].contains(&format!("field `{field}`")) && lines[0].contains(lacks),
            "`{field}` with {text:?}: {lines:?}"
        );
    }
    // `detail` with neither base names both.
    let lines = refused(&entries, &bases_with("detail", Some("a union")));
    assert!(
        lines.len() == 1 && lines[0].contains(BODY_BASE) && lines[0].contains(PAIR_BASE),
        "`detail` with neither: {lines:?}"
    );
    // A layout without `m1` is not checked for it.
    let without: Vec<Entry> = entries.iter().filter(|e| e.name != "m1").cloned().collect();
    assert_eq!(
        refused(&without, &bases_with("m1", None)),
        Vec::<String>::new(),
        "a layout without `m1`"
    );
}

#[test]
fn index_base_refuses_an_unstated_base() {
    check_refusals(index_refused);
}

negative_control!(
    index_base_refuses_an_unstated_base,
    "a check that only looks for a row, not its text, fails",
    expected = "`r` with Some",
    check_refusals(|entries, bases| {
        INDEXED
            .iter()
            .filter(|(f, _)| entries.iter().any(|e| e.name == *f))
            .filter(|(f, _)| !bases.iter().any(|(b, _)| b == f))
            .map(|(f, _)| format!("field `{f}` {BODY_BASE}"))
            .collect()
    })
);

/// Checks the key: each indexed field has its row, in `INDEXED`'s order, stating the bases its kind needs, and no
/// other field has one.
fn check_key(bases: &[(&str, &str)]) {
    assert_eq!(bases.len(), INDEXED.len(), "one row per indexed field");
    for ((field, kind), (row, text)) in INDEXED.iter().zip(bases) {
        assert_eq!(field, row, "the rows' order");
        let wanted: &[&str] = match kind {
            Indexed::Body => &[BODY_BASE],
            Indexed::Pair => &[PAIR_BASE],
            Indexed::BodyOrPair => &[BODY_BASE, PAIR_BASE],
        };
        for w in wanted {
            assert!(text.contains(w), "`{field}`'s base `{text}` lacks \"{w}\"");
        }
    }
    assert_eq!(BODY_BASE, "body index, 0-based");
    assert_eq!(
        PAIR_BASE,
        "pair id, 0-based; pair k is the side opposite body k"
    );
}

#[test]
fn index_base_key_states_each_base() {
    check_key(&INDEX_BASES);
    // The corpus's fields (dd_generation_root §3.8): `detail` both, `dmin_pair` a pair, `m0 m1 m2` bodies.
    let kind = |f: &str| INDEXED.iter().find(|(g, _)| *g == f).map(|&(_, k)| k);
    assert_eq!(kind("detail"), Some(Indexed::BodyOrPair));
    assert_eq!(kind("dmin_pair"), Some(Indexed::Pair));
    for m in ["m0", "m1", "m2", "r", "p", "r_sh", "p_sh"] {
        assert_eq!(kind(m), Some(Indexed::Body), "{m}");
    }
}

negative_control!(
    index_base_key_states_each_base,
    "a key giving `dmin_pair` a body index fails",
    expected = "`dmin_pair`'s base",
    check_key(&bases_with("dmin_pair", Some(BODY_BASE)))
);

/// Checks that generation through `generate`, with the export decoder, is refused for a dropped base and generates
/// with the ledger's key; and that without the export decoder it does not check the bases.
fn check_generation(
    generate: impl Fn(&[(&str, &str)], &[gen::Emitter]) -> Result<usize, GenError>,
) {
    let bases = bases_with("m0", None);
    match generate(&bases, &[export::emit]) {
        Err(GenError::Unread(lines)) => {
            assert!(
                lines.len() == 1 && lines[0].contains("field `m0`"),
                "{lines:?}"
            )
        }
        other => panic!("generation without `m0`'s base was not refused: {other:?}"),
    }
    assert_eq!(generate(&INDEX_BASES, &[export::emit]), Ok(1));
    assert_eq!(
        generate(&bases, &[gen::rust::emit]).map(|n| n > 0),
        Ok(true),
        "the Rust emitter alone does not read the bases"
    );
}

#[test]
fn index_base_generation_is_refused() {
    check_generation(|bases, emitters| {
        generate_with_index_bases(bases, &ledger::layout(), emitters).map(|g| g.len())
    });
    // `generate` reads the ledger's own key.
    assert_eq!(
        gen::generate(&ledger::layout(), &[export::emit]).map(|g| g.len()),
        Ok(1)
    );
}

negative_control!(
    index_base_generation_is_refused,
    "a generation that ignores the bases fails",
    expected = "generation without `m0`'s base was not refused",
    check_generation(|_, emitters| { gen::generate(&ledger::layout(), emitters).map(|g| g.len()) })
);
