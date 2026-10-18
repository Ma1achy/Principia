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
//! - A body-indexed ledger entry without the text fails generation, naming the field; so does one whose text names a
//!   1-based base (`qa_m2_33_export_refuses_*`).
//!
//! Each test has its negative control (R-176).
// The file name `qa_TASK-M2-33` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use engine::export::{decode, Decoded, INDEX_BASES};
use kernel::payload::*;
use ledger::gen::{export, generate, generate_with_index_bases, GenError};
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

fn check_refuses(gen: impl Fn(&[(&str, &str)]) -> Result<(), GenError>) {
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
        let bases: Vec<(&str, &str)> = ledger::payload::INDEX_BASES
            .iter()
            .map(|&(f, t)| if f == field { (f, text) } else { (f, t) })
            .collect();
        match gen(&bases) {
            Err(GenError::Unread(lines)) => assert!(
                lines.iter().any(|l| l.contains(&format!("`{field}`"))),
                "the refusal does not name `{field}`: {lines:?}"
            ),
            other => panic!("`{field}` with the base `{text}` generated: {other:?}"),
        }
    }
    // Every base dropped: each indexed field is refused.
    match gen(&[]) {
        Err(GenError::Unread(lines)) => {
            for (field, _) in INDEXED {
                assert!(
                    lines.iter().any(|l| l.contains(&format!("`{field}`"))),
                    "with no bases, `{field}` is not refused: {lines:?}"
                );
            }
        }
        other => panic!("with no bases, generation went ahead: {other:?}"),
    }
}

#[test]
fn qa_m2_33_export_refuses_a_missing_base() {
    check_refuses(|b| generate_with_index_bases(b, &ledger::layout(), &[export::emit]).map(|_| ()));
}

negative_control!(
    qa_m2_33_export_refuses_a_missing_base,
    "a generator that never checks the bases fails",
    expected = "`m2` with the base `body 2's mass` generated",
    check_refuses(|_| generate(&ledger::layout(), &[export::emit]).map(|_| ()))
);
