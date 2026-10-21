//! QA tests for TASK-M2-33 (REQ-GEN-036; R-449, R-22; dd_generation_root §3.8), written from the requirement, not
//! from the implementation:
//!
//! - The export decoder keeps the internal 0-based ids: a sample packed with a collision of bodies `a` and `b` exports
//!   `detail` = the pair opposite the third body, `3 − a − b` (R-22, worked here from the rule, not the ledger's pair
//!   map), so bodies 0 and 1 write 2; an escape of body `k` exports `k`, body 0 writing 0; `dmin_pair` exports its pair
//!   id and its unset 3 (`qa_m2_33_export_ids_*`).
//! - The generated decoder states the base of each body- or pair-indexed field §3.8 names (`detail` under escape and
//!   collision, `dmin_pair`, `m0 m1 m2`, and the per-body vectors), in its schema and in the member's doc, with
//!   R-449's words: "body index, 0-based" and "pair id, 0-based; pair k is the side opposite body k"; `detail` both
//!   (`qa_m2_33_export_header_*`).
//! - The header is generated from the ledger, not hand-written: the checked-in decoder is byte-for-byte what the
//!   ledger's generator emits now (`qa_m2_33_export_generated_*`).
//! - A ledger entry marked body- or pair-indexed (its §3.8 index-base key) without the text fails generation, naming
//!   the field; so does one whose text names a 1-based base, and so does every marked entry at once with its text
//!   emptied (`qa_m2_33_export_refuses_*`).
//! - The check follows the entry's own key, not a list of names: an indexed entry renamed is still refused under its
//!   new name, and any other per-body or per-pair field marked indexed without the text is refused too
//!   (`qa_m2_33_export_key_*`).
//!
//! Each test has its negative control (R-176).
// The file name `qa_TASK-M2-33` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use engine::export::{decode, Decoded, INDEX_BASES};
use kernel::payload::*;
use ledger::gen::{export, generate, GenError};
use ledger::schema::{EntryBuilder, IndexBase, Indexed, Ledger, Location};
use validation::negative_control;

/// R-449's words.
const BODY: &str = "body index, 0-based";
const PAIR: &str = "pair id, 0-based; pair k is the side opposite body k";

/// The decoder exactly as checked in.
const SOURCE: &str = include_str!("../src/export/generated.rs");

// ── The ids stay 0-based ────────────────────────────────────────────────────────────────────────────────────────────

/// The decoded sample of a payload with `state`, `detail` and `dmin_pair`, every other descriptor bit set (the
/// saturated flag and last symbol), so a decoder reading the wrong bits shows.
fn export_of(state: u32, detail: u32, dmin_pair: u32) -> Decoded {
    let mut s = SimStateFTLE::default();
    s.packed_a = set_saturated(s.packed_a, true);
    s.packed_a = set_last_symbol(s.packed_a, 3);
    s.packed_a = set_state(s.packed_a, state);
    s.packed_a = set_detail(s.packed_a, detail);
    s.packed_a = set_dmin_pair(s.packed_a, dmin_pair);
    let params = ReadParams {
        dt_macro: 0.01,
        delta_0: 1e-6,
        n_renorm: 16,
        horizon_steps: 1000,
    };
    decode(&s, [0; 4], &ICDescriptor::default(), &params)
}

/// R-22: the pair of bodies `a` and `b` is the side opposite the third body, so its id is the third body's index.
fn pair_id(a: u32, b: u32) -> u32 {
    3 - a - b
}

fn check_ids(export: impl Fn(u32, u32, u32) -> (u32, u32)) {
    // The requirement's two cases, literally.
    assert_eq!(
        export(STATE_COLLISION, 2, 2).0,
        2,
        "a collision of bodies 0 and 1 does not export detail 2"
    );
    assert_eq!(
        export(STATE_ESCAPE, 0, 3).0,
        0,
        "an escape of body 0 does not export detail 0"
    );
    for (a, b) in [(0, 1), (0, 2), (1, 2)] {
        let k = pair_id(a, b);
        assert_eq!(
            export(STATE_COLLISION, k, k),
            (k, k),
            "a collision of bodies {a} and {b} (pair {k}) and its dmin_pair"
        );
    }
    for k in 0..3 {
        assert_eq!(
            export(STATE_ESCAPE, k, 3),
            (k, 3),
            "an escape of body {k}, dmin_pair unset"
        );
    }
    assert_eq!(export(STATE_ESCAPE, 3, 0), (3, 0), "a triple ejection");
    assert_eq!(export(STATE_COLLISION, 3, 1), (3, 1), "a triple collision");
}

#[test]
fn qa_m2_33_export_ids_stay_0_based() {
    check_ids(|s, d, p| {
        let x = export_of(s, d, p);
        assert_eq!(x.state, s, "the exported state");
        (x.detail, x.dmin_pair)
    });
}

negative_control!(
    qa_m2_33_export_ids_stay_0_based,
    "an export that writes the collision's pair as shown, by the lower body 1-based, fails",
    expected = "a collision of bodies 0 and 1 does not export detail 2",
    check_ids(|s, d, p| {
        let x = export_of(s, d, p);
        if s == STATE_COLLISION && d == 2 {
            // Bodies 0 and 1, shown "1–2": a pair written by its first shown body.
            (1, x.dmin_pair)
        } else {
            (x.detail, x.dmin_pair)
        }
    })
);

// ── The header states the base ──────────────────────────────────────────────────────────────────────────────────────

/// §3.8's body- and pair-indexed fields the decoder holds, with the bases R-449 asks of each.
const INDEXED: [(&str, &[&str]); 9] = [
    ("detail", &[BODY, PAIR]),
    ("dmin_pair", &[PAIR]),
    ("m0", &[BODY]),
    ("m1", &[BODY]),
    ("m2", &[BODY]),
    ("r", &[BODY]),
    ("p", &[BODY]),
    ("r_sh", &[BODY]),
    ("p_sh", &[BODY]),
];

/// Checks the schema and the source: each indexed field's schema text holds its bases, and the doc block just above
/// its member in `source` holds them too.
fn check_header(schema: &[(&str, &str)], source: &str) {
    let lines: Vec<&str> = source.lines().collect();
    for (field, bases) in INDEXED {
        let text = schema
            .iter()
            .find(|(f, _)| *f == field)
            .map(|&(_, t)| t)
            .unwrap_or_else(|| panic!("the decoder's schema states no base for `{field}`"));
        let at = lines
            .iter()
            .position(|l| l.trim().starts_with(&format!("pub {field}:")))
            .unwrap_or_else(|| panic!("the decoder has no member `{field}`"));
        let doc: String = lines[..at]
            .iter()
            .rev()
            .take_while(|l| l.trim_start().starts_with("///"))
            .copied()
            .collect::<Vec<_>>()
            .join(" ");
        for base in bases {
            assert!(
                text.contains(base),
                "`{field}`'s schema text `{text}` does not state \"{base}\""
            );
            assert!(
                doc.contains(base),
                "`{field}`'s doc `{doc}` does not state \"{base}\""
            );
        }
    }
}

#[test]
fn qa_m2_33_export_header_states_the_base() {
    check_header(&INDEX_BASES, SOURCE);
}

negative_control!(
    qa_m2_33_export_header_states_the_base,
    "a decoder whose dmin_pair doc drops \"0-based\" fails",
    expected = "`dmin_pair`'s doc",
    {
        let source = SOURCE.replace(
            "/// Index base: pair id, 0-based; pair k is the side opposite body k; 3 unset.",
            "/// Index base: the pair; 3 unset.",
        );
        assert_ne!(source, SOURCE, "the control's edit did not apply");
        check_header(&INDEX_BASES, &source);
    }
);

// ── Generated, not hand-written ─────────────────────────────────────────────────────────────────────────────────────

/// The decoder the ledger's generator emits now.
fn emitted() -> String {
    generate(&ledger::layout(), &[export::emit])
        .expect("the export decoder generates")
        .into_iter()
        .find(|g| g.path.ends_with("export/generated.rs"))
        .expect("the generator emits the export decoder")
        .contents
}

fn check_generated(emitted: &str, checked_in: &str) {
    assert!(
        emitted == checked_in,
        "the checked-in decoder is not the generator's output"
    );
}

#[test]
fn qa_m2_33_export_generated_from_the_ledger() {
    check_generated(&emitted(), SOURCE);
}

negative_control!(
    qa_m2_33_export_generated_from_the_ledger,
    "a decoder whose m0 base was written by hand fails",
    expected = "not the generator's output",
    {
        let edited = SOURCE.replace(
            "/// Index base: body index, 0-based: body 0's mass.",
            "/// Index base: body index, 0-based: the first mass.",
        );
        assert_ne!(edited, SOURCE, "the control's edit did not apply");
        check_generated(&emitted(), &edited);
    }
);

// ── A missing base fails generation ─────────────────────────────────────────────────────────────────────────────────

/// The payload layout with `edit` applied to the entry named `field`.
fn layout_with(field: &str, edit: impl FnOnce(&mut EntryBuilder)) -> Ledger {
    let mut l = ledger::layout();
    let e = l
        .entries
        .iter_mut()
        .find(|e| e.name == Some(field))
        .unwrap_or_else(|| panic!("the payload ledger has no entry `{field}`"));
    edit(e);
    l
}

/// The payload layout with `field`'s index-base text replaced by `text`, its kind of index kept.
fn text_of(field: &str, text: &'static str) -> Ledger {
    layout_with(field, |e| {
        let key = e
            .index_base
            .unwrap_or_else(|| panic!("`{field}` is not marked body- or pair-indexed"));
        e.index_base = Some(IndexBase { text, ..key });
    })
}

/// The refusal lines generating `layout` gives, or none when it generates.
fn refusals(gen: &impl Fn(&Ledger) -> Result<(), GenError>, layout: &Ledger) -> Vec<String> {
    match gen(layout) {
        Ok(()) => vec![],
        Err(GenError::Unread(lines)) => lines,
        Err(other) => panic!("generation failed other than by refusing a field: {other:?}"),
    }
}

fn check_refuses(gen: impl Fn(&Ledger) -> Result<(), GenError>) {
    assert_eq!(
        gen(&ledger::layout()),
        Ok(()),
        "the payload ledger generates"
    );
    for (field, text) in [
        ("m2", "body 2's mass"),
        ("m0", "body index, 1-based"),
        ("dmin_pair", "pair id; pair k is the side opposite body k"),
        (
            "detail",
            "under collision, pair id, 0-based; pair k is the side opposite body k",
        ),
        ("p", "components 2j and 2j + 1"),
    ] {
        match gen(&text_of(field, text)) {
            Err(GenError::Unread(lines)) => assert!(
                lines.iter().any(|l| l.contains(&format!("`{field}`"))),
                "the refusal does not name `{field}`: {lines:?}"
            ),
            other => panic!("`{field}` with the base `{text}` generated: {other:?}"),
        }
    }
    // Every marked entry at once, its text emptied: each is refused, named. (In place of the side table's "every base
    // dropped": the base now lives on the entry, so an entry still marked indexed with no text is the whole case.)
    let mut l = ledger::layout();
    for e in &mut l.entries {
        if let Some(key) = e.index_base {
            e.index_base = Some(IndexBase { text: "", ..key });
        }
    }
    match gen(&l) {
        Err(GenError::Unread(lines)) => {
            for (field, _) in INDEXED {
                assert!(
                    lines.iter().any(|l| l.contains(&format!("`{field}`"))),
                    "with every text emptied, `{field}` is not refused: {lines:?}"
                );
            }
        }
        other => panic!("with every text emptied, generation went ahead: {other:?}"),
    }
}

/// Generates the export decoder from `layout`, keeping only whether it was refused.
fn generated(layout: &Ledger) -> Result<(), GenError> {
    generate(layout, &[export::emit]).map(|_| ())
}

#[test]
fn qa_m2_33_export_refuses_a_missing_base() {
    check_refuses(generated);
}

negative_control!(
    qa_m2_33_export_refuses_a_missing_base,
    "a generator that never reads the entries' index-base keys fails",
    expected = "`m2` with the base `body 2's mass` generated",
    check_refuses(|_| generated(&ledger::layout()))
);

// ── The check follows the entry's key ───────────────────────────────────────────────────────────────────────────────

/// REQ-GEN-036's "any per-body or per-pair field": the check reads each entry's own key. `p_sh` renamed `qa_p_sh_moved`
/// (and so read by the derived fields that read it) keeps its key: with its text, renaming adds no refusal naming it; with a text that drops "0-based", a refusal names
/// it under the new name. A field no list names, `q_mass`, marked body-indexed without the text is refused; marked
/// with R-449's words it generates. `detail`'s key narrowed to a body index while its text states only the pair base
/// is refused (the kind decides which bases the text must state).
fn check_key(gen: impl Fn(&Ledger) -> Result<(), GenError>) {
    let renamed = |text: Option<&'static str>| {
        let mut l = layout_with("p_sh", |e| {
            e.name = Some("qa_p_sh_moved");
            if let Some(text) = text {
                let key = e.index_base.expect("`p_sh` is marked body-indexed");
                e.index_base = Some(IndexBase { text, ..key });
            }
        });
        // The derived fields that read `p_sh` read it under its new name.
        for e in &mut l.entries {
            if let Some(Location::Derived { from }) = &mut e.location {
                for f in from.iter_mut().filter(|f| **f == "p_sh") {
                    *f = "qa_p_sh_moved";
                }
            }
        }
        l
    };
    let names = |lines: &[String]| -> usize {
        lines
            .iter()
            .filter(|l| l.contains("`qa_p_sh_moved`"))
            .count()
    };
    let kept = names(&refusals(&gen, &renamed(None)));
    let bad = names(&refusals(&gen, &renamed(Some("components 2j and 2j + 1"))));
    assert!(
        bad > kept,
        "the renamed indexed entry `qa_p_sh_moved` without the text is not refused ({kept} lines name it either way)"
    );

    let q = |text: &'static str| {
        layout_with("q_mass", |e| {
            assert_eq!(e.index_base, None, "`q_mass` is already marked");
            e.index_base = Some(IndexBase {
                indexed: Indexed::Body,
                text,
            });
        })
    };
    assert_eq!(
        gen(&q("body index, 0-based: the qa stand-in")),
        Ok(()),
        "`q_mass` marked with R-449's words"
    );
    assert!(
        refusals(&gen, &q("the mass ratio"))
            .iter()
            .any(|l| l.contains("`q_mass`")),
        "`q_mass` marked body-indexed without the text is not refused"
    );

    let narrowed = layout_with("detail", |e| {
        e.index_base = Some(IndexBase {
            indexed: Indexed::Body,
            text: PAIR,
        });
    });
    assert!(
        refusals(&gen, &narrowed)
            .iter()
            .any(|l| l.contains("`detail`")),
        "`detail` marked body-indexed with only the pair base is not refused"
    );
}

#[test]
fn qa_m2_33_export_key_follows_the_entry() {
    check_key(generated);
}

negative_control!(
    qa_m2_33_export_key_follows_the_entry,
    "a generator that checks a fixed list of names, not the entries' keys, fails",
    expected = "the renamed indexed entry `qa_p_sh_moved` without the text is not refused",
    check_key(|l: &Ledger| {
        // The side table's way: only the names it lists are checked, so a renamed entry escapes.
        const LISTED: [&str; 9] = [
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
        let mut l = l.clone();
        for e in &mut l.entries {
            if !e.name.is_some_and(|n| LISTED.contains(&n)) {
                e.index_base = None;
            }
        }
        generated(&l)
    })
);
