//! The ledger's index bases (dd_generation_root §3.8; R-449; REQ-GEN-036), in the ledger's own tests: the entry's
//! index-base key (`schema::IndexBase`), which states the base of a body- or pair-indexed field's ids on the entry
//! itself; the payload ledger's keys; the check that refuses generation of the export decoder for an entry whose key
//! does not state its base (`gen::export::index_refused`); and the schema version, which the key does not move. The
//! engine's `export_index_base_*` check the generated decoder. Each test has a registered negative control (R-176).

use ledger::gen::export::{self, index_refused};
use ledger::gen::{self, GenError};
use ledger::schema::{Entry, IndexBase, Indexed, Ledger, BODY_BASE, PAIR_BASE};
use ledger::version::{schema_version, Hashed};
use validation::negative_control;

/// The payload ledger's entries.
fn entries() -> Vec<Entry> {
    gen::validate(&ledger::layout()).unwrap_or_else(|e| panic!("{e}"))
}

/// `entries` with `field`'s index-base text replaced by `text`.
fn with_text(entries: &[Entry], field: &str, text: &'static str) -> Vec<Entry> {
    let mut out = entries.to_vec();
    let e = out
        .iter_mut()
        .find(|e| e.name == field)
        .unwrap_or_else(|| panic!("no entry `{field}`"));
    let base = e
        .index_base
        .unwrap_or_else(|| panic!("`{field}` has no index base"));
    e.index_base = Some(IndexBase { text, ..base });
    out
}

/// Checks the refusals `refused` gives: none for the ledger's own keys; for an entry whose key does not state its
/// base, one line naming the field and the base it lacks, whatever the field is named, a new entry's too.
fn check_refusals(refused: impl Fn(&[Entry]) -> Vec<String>) {
    let entries = entries();
    assert_eq!(
        refused(&entries),
        Vec::<String>::new(),
        "the ledger's own keys"
    );
    for (field, text, lacks) in [
        ("m1", "body 1's mass", BODY_BASE),
        ("r", "components 2j and 2j + 1 are body j's", BODY_BASE),
        ("dmin_pair", BODY_BASE, PAIR_BASE),
        ("detail", BODY_BASE, PAIR_BASE),
        ("detail", PAIR_BASE, BODY_BASE),
    ] {
        let lines = refused(&with_text(&entries, field, text));
        assert_eq!(lines.len(), 1, "`{field}` with `{text}`: {lines:?}");
        assert!(
            lines[0].contains(&format!("field `{field}`")) && lines[0].contains(lacks),
            "`{field}` with `{text}`: {lines:?}"
        );
    }
    // `detail` with neither base names both.
    let lines = refused(&with_text(&entries, "detail", "a union"));
    assert!(
        lines.len() == 1 && lines[0].contains(BODY_BASE) && lines[0].contains(PAIR_BASE),
        "`detail` with neither: {lines:?}"
    );
    // The key travels with the entry: renamed, `m1` is still checked, under its new name.
    let mut renamed = with_text(&entries, "m1", "body 1's mass");
    renamed.iter_mut().find(|e| e.name == "m1").unwrap().name = "mass_1";
    let lines = refused(&renamed);
    assert!(
        lines.len() == 1 && lines[0].contains("field `mass_1`"),
        "the renamed `mass_1`: {lines:?}"
    );
    // A new indexed entry is checked by its own key.
    let mut added = entries.clone();
    let mut extra = entries.iter().find(|e| e.name == "S").unwrap().clone();
    extra.name = "per_pair";
    extra.index_base = Some(IndexBase {
        indexed: Indexed::Pair,
        text: "pair id",
    });
    added.push(extra);
    let lines = refused(&added);
    assert!(
        lines.len() == 1 && lines[0].contains("field `per_pair`"),
        "a new pair-indexed entry: {lines:?}"
    );
}

#[test]
fn index_base_refuses_an_unstated_base() {
    check_refusals(index_refused);
}

negative_control!(
    index_base_refuses_an_unstated_base,
    "a check keyed by a fixed list of field names fails",
    expected = "the renamed `mass_1`",
    check_refusals(|entries| {
        let named = [
            "r",
            "p",
            "r_sh",
            "p_sh",
            "detail",
            "dmin_pair",
            "m0",
            "m1",
            "m2",
        ];
        index_refused(entries)
            .into_iter()
            .filter(|l| named.iter().any(|n| l.contains(&format!("field `{n}`"))))
            .collect()
    })
);

/// Checks the payload ledger's keys: exactly §3.8's body- and pair-indexed fields carry one, each of its kind, and each
/// states the bases its kind needs, R-449's words.
fn check_keys(entries: &[Entry]) {
    let keyed: Vec<(&str, Indexed)> = entries
        .iter()
        .filter_map(|e| Some((e.name, e.index_base?.indexed)))
        .collect();
    assert_eq!(
        keyed,
        [
            ("r", Indexed::Body),
            ("p", Indexed::Body),
            ("r_sh", Indexed::Body),
            ("p_sh", Indexed::Body),
            ("detail", Indexed::BodyOrPair),
            ("dmin_pair", Indexed::Pair),
            ("m0", Indexed::Body),
            ("m1", Indexed::Body),
            ("m2", Indexed::Body),
        ],
        "the indexed fields and their kinds"
    );
    for e in entries {
        if let Some(base) = e.index_base {
            for w in base.indexed.bases() {
                assert!(
                    base.text.contains(w),
                    "`{}`'s base `{}` lacks \"{w}\"",
                    e.name,
                    base.text
                );
            }
        }
    }
    assert_eq!(BODY_BASE, "body index, 0-based");
    assert_eq!(
        PAIR_BASE,
        "pair id, 0-based; pair k is the side opposite body k"
    );
    assert_eq!(Indexed::Body.bases(), [BODY_BASE]);
    assert_eq!(Indexed::Pair.bases(), [PAIR_BASE]);
    assert_eq!(Indexed::BodyOrPair.bases(), [BODY_BASE, PAIR_BASE]);
}

#[test]
fn index_base_key_states_each_base() {
    check_keys(&entries());
}

negative_control!(
    index_base_key_states_each_base,
    "a ledger giving `dmin_pair` a body index fails",
    expected = "the indexed fields and their kinds",
    {
        let mut es = entries();
        let e = es.iter_mut().find(|e| e.name == "dmin_pair").unwrap();
        e.index_base = Some(IndexBase {
            indexed: Indexed::Body,
            text: BODY_BASE,
        });
        check_keys(&es);
    }
);

/// Checks [`IndexBase::missing`] through `missing` at its boundary: a text holding the base exactly states it; one a
/// character short does not; `detail`'s kind needs both.
fn check_missing(missing: impl Fn(Indexed, &'static str) -> Vec<&'static str>) {
    assert_eq!(missing(Indexed::Body, BODY_BASE), Vec::<&str>::new());
    assert_eq!(
        missing(Indexed::Body, "body index, 0-base"),
        [BODY_BASE],
        "a text a character short"
    );
    assert_eq!(missing(Indexed::Pair, PAIR_BASE), Vec::<&str>::new());
    assert_eq!(missing(Indexed::Pair, BODY_BASE), [PAIR_BASE]);
    let both = "body index, 0-based; pair id, 0-based; pair k is the side opposite body k";
    assert_eq!(missing(Indexed::BodyOrPair, both), Vec::<&str>::new());
    assert_eq!(missing(Indexed::BodyOrPair, PAIR_BASE), [BODY_BASE]);
    assert_eq!(missing(Indexed::BodyOrPair, ""), [BODY_BASE, PAIR_BASE]);
}

#[test]
fn index_base_missing_at_its_boundary() {
    check_missing(|indexed, text| IndexBase { indexed, text }.missing());
}

negative_control!(
    index_base_missing_at_its_boundary,
    "a check that a text a character short states the base fails",
    expected = "a text a character short",
    check_missing(|indexed, text| {
        IndexBase { indexed, text }
            .missing()
            .into_iter()
            .filter(|base| !base.starts_with(text))
            .collect()
    })
);

/// The payload layout with `field`'s index-base text replaced by `text`.
fn layout_with(field: &str, text: &'static str) -> Ledger {
    let mut l = ledger::layout();
    let b = l
        .entries
        .iter_mut()
        .find(|b| b.name == Some(field))
        .unwrap_or_else(|| panic!("no entry `{field}`"));
    let base = b.index_base.expect("an index base");
    b.index_base = Some(IndexBase { text, ..base });
    l
}

/// Checks that generation through `generate`, with the export decoder, is refused for an entry whose key does not
/// state its base, and generates with the ledger's own; and that without the export decoder it does not check them.
fn check_generation(generate: impl Fn(&Ledger, &[gen::Emitter]) -> Result<usize, GenError>) {
    let bad = layout_with("m0", "body 0's mass");
    match generate(&bad, &[export::emit]) {
        Err(GenError::Unread(lines)) => {
            assert!(
                lines.len() == 1 && lines[0].contains("field `m0`"),
                "{lines:?}"
            )
        }
        other => panic!("generation without `m0`'s base was not refused: {other:?}"),
    }
    assert_eq!(generate(&ledger::layout(), &[export::emit]), Ok(1));
    assert_eq!(
        generate(&bad, &[gen::rust::emit]).map(|n| n > 0),
        Ok(true),
        "the Rust emitter alone does not read the bases"
    );
}

#[test]
fn index_base_generation_is_refused() {
    check_generation(|l, emitters| gen::generate(l, emitters).map(|g| g.len()));
}

negative_control!(
    index_base_generation_is_refused,
    "a generation that ignores the entries' keys fails",
    expected = "generation without `m0`'s base was not refused",
    check_generation(|_, emitters| gen::generate(&ledger::layout(), emitters).map(|g| g.len()))
);

/// The schema version of `entries` over the payload's words and structs.
fn version(entries: &[Entry]) -> u64 {
    let l = ledger::layout();
    let structs = ledger::payload::structs();
    schema_version(&Hashed::payload(&l.words, entries, &structs)).unwrap_or_else(|e| panic!("{e}"))
}

/// Checks that the index-base keys do not move the schema version: the entries with every key dropped, and with a
/// text changed, hash as the ledger does (R-449: "the schema version does not move").
fn check_unhashed(version: impl Fn(&[Entry]) -> u64) {
    let es = entries();
    let dropped: Vec<Entry> = es
        .iter()
        .cloned()
        .map(|mut e| {
            e.index_base = None;
            e
        })
        .collect();
    assert_eq!(version(&dropped), version(&es), "the keys dropped");
    let changed = with_text(&es, "detail", "a union");
    assert_eq!(version(&changed), version(&es), "`detail`'s text changed");
}

#[test]
fn index_base_does_not_move_the_schema_version() {
    check_unhashed(version);
}

negative_control!(
    index_base_does_not_move_the_schema_version,
    "a version that hashed the keys moves",
    expected = "the keys dropped",
    check_unhashed(|es| {
        let keyed = es.iter().filter(|e| e.index_base.is_some()).count() as u64;
        version(es) ^ keyed
    })
);
