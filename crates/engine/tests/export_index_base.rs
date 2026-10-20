//! The export decoder's index bases (dd_generation_root §3.8; R-449; REQ-GEN-036): the generated decoder
//! (`engine::export`, from `ledger::gen::export`) keeps the body and pair ids 0-based, as the payload holds them, and its
//! schema says so, "body index, 0-based" for each body-indexed field and "pair id, 0-based; pair k is the side opposite
//! body k" for `detail` under collision and for `dmin_pair`, from each ledger entry's index-base key
//! (`ledger::schema::IndexBase`), not written by hand; an exported collision of bodies 0 and 1 writes `detail` 2 and an escape of body 0 writes 0; and a
//! body- or pair-indexed ledger entry without the text fails generation. Each test has a registered negative control
//! (R-176).

use engine::export::{decode, INDEX_BASES};
use kernel::payload::*;
use ledger::gen::{export, generate, GenError};
use ledger::schema::{IndexBase, Ledger};
use validation::negative_control;

/// R-449's words for a body index.
const BODY: &str = "body index, 0-based";
/// R-449's words for a pair id.
const PAIR: &str = "pair id, 0-based; pair k is the side opposite body k";

/// The body- and pair-indexed fields and the bases each must state: the masses and the stored vectors a body index,
/// `dmin_pair` a pair id, `detail` both (a body under escape, a pair under collision).
const WANT: [(&str, &[&str]); 9] = [
    ("r", &[BODY]),
    ("p", &[BODY]),
    ("r_sh", &[BODY]),
    ("p_sh", &[BODY]),
    ("detail", &[BODY, PAIR]),
    ("dmin_pair", &[PAIR]),
    ("m0", &[BODY]),
    ("m1", &[BODY]),
    ("m2", &[BODY]),
];

/// The generated decoder's source, its header and member docs.
const SOURCE: &str = include_str!("../src/export/generated.rs");

/// Checks that `schema` and `source` state each field's bases: the schema's text for it holds each, and the member's
/// doc in `source`, the line just above `pub <field>:`, is that text.
fn check_header(schema: &[(&str, &str)], source: &str) {
    for (field, bases) in WANT {
        let text = schema
            .iter()
            .find(|(f, _)| *f == field)
            .map(|&(_, t)| t)
            .unwrap_or_else(|| panic!("the schema has no base for `{field}`"));
        for base in bases {
            assert!(
                text.contains(base),
                "`{field}`'s base `{text}` does not state \"{base}\""
            );
        }
        let lines: Vec<&str> = source.lines().collect();
        let at = lines
            .iter()
            .position(|l| l.trim_start().starts_with(&format!("pub {field}:")))
            .unwrap_or_else(|| panic!("no member `{field}`"));
        assert_eq!(
            lines[at - 1].trim(),
            format!("/// Index base: {text}."),
            "`{field}`'s doc in the decoder's header"
        );
    }
    // No other field carries a base.
    assert_eq!(schema.len(), WANT.len(), "the schema's fields");
}

#[test]
fn export_index_base_header_states_the_base() {
    check_header(&INDEX_BASES, SOURCE);
    // The schema is the ledger entries' keys, in the ledger's order, generated, not written by hand.
    let keys: Vec<(&str, &str)> = ledger::gen::validate(&ledger::layout())
        .expect("the payload ledger validates")
        .iter()
        .filter_map(|e| Some((e.name, e.index_base?.text)))
        .collect();
    assert_eq!(INDEX_BASES.to_vec(), keys);
}

negative_control!(
    export_index_base_header_states_the_base,
    "a header whose `m1` base does not say \"0-based\" fails",
    expected = "`m1`'s base",
    {
        let schema: Vec<(&str, &str)> = INDEX_BASES
            .iter()
            .map(|&(f, t)| {
                if f == "m1" {
                    (f, "body 1's mass")
                } else {
                    (f, t)
                }
            })
            .collect();
        check_header(&schema, SOURCE);
    }
);

/// The pair id of the collision of bodies `a` and `b`, by the ledger's pair map: the side opposite the third body.
fn pair_of(a: u32, b: u32) -> u32 {
    let pairs = ledger::payload::pair_bodies();
    (0..3)
        .find(|&k| {
            let [p, q] = pairs[k as usize];
            (p.min(q), p.max(q)) == (a.min(b), a.max(b))
        })
        .unwrap()
}

/// Checks the ids an export writes: through `export_detail`, a collision of bodies 0 and 1 writes `detail` 2, its
/// `dmin_pair` too, and an escape of body 0 writes 0, all 0-based.
fn check_ids(export_detail: impl Fn(u32, u32, u32) -> (u32, u32)) {
    assert_eq!(pair_of(0, 1), 2, "bodies 0 and 1 are pair 2");
    assert_eq!(
        export_detail(STATE_COLLISION, pair_of(0, 1), pair_of(0, 1)),
        (2, 2),
        "a collision of bodies 0 and 1"
    );
    assert_eq!(
        export_detail(STATE_ESCAPE, 0, 3),
        (0, 3),
        "an escape of body 0"
    );
}

/// The `detail` and `dmin_pair` the decoder writes for a sample with `state`, `detail` and `dmin_pair`.
fn exported(state: u32, detail: u32, dmin_pair: u32) -> (u32, u32) {
    let mut s = SimStateFTLE::default();
    s.packed_a = set_state(s.packed_a, state);
    s.packed_a = set_detail(s.packed_a, detail);
    s.packed_a = set_dmin_pair(s.packed_a, dmin_pair);
    let params = ReadParams {
        dt_macro: 0.01,
        delta_0: 1e-6,
        n_renorm: 16,
        horizon_steps: 1000,
    };
    let d = decode(&s, [0; 4], &ICDescriptor::default(), &params);
    (d.detail, d.dmin_pair)
}

#[test]
fn export_index_base_ids_stay_0_based() {
    check_ids(exported);
}

negative_control!(
    export_index_base_ids_stay_0_based,
    "an export writing the shown, 1-based body fails",
    expected = "an escape of body 0",
    check_ids(|s, d, p| {
        let (d, p) = exported(s, d, p);
        if s == STATE_ESCAPE {
            (d + 1, p)
        } else {
            (d, p)
        }
    })
);

/// The payload layout with `field`'s index-base key replaced by `text`, or dropped where `text` is `None`.
fn layout_with(field: &str, text: Option<&'static str>) -> Ledger {
    let mut l = ledger::layout();
    let b = l
        .entries
        .iter_mut()
        .find(|b| b.name == Some(field))
        .unwrap_or_else(|| panic!("no entry `{field}`"));
    let base = b.index_base.expect("an index base");
    b.index_base = text.map(|text| IndexBase { text, ..base });
    l
}

/// Checks that `generate`, given a layout, refuses each indexed entry whose key does not state its base, naming the
/// field, and generates from the ledger's own; an entry whose key is dropped is no longer indexed, and generates.
fn check_refusal(generate: impl Fn(&Ledger) -> Result<(), GenError>) {
    assert_eq!(generate(&ledger::layout()), Ok(()));
    for (field, text) in [
        ("m1", "body 1's mass"),
        ("r", "body j's components"),
        ("dmin_pair", BODY),
        ("detail", BODY),
        ("detail", PAIR),
    ] {
        match generate(&layout_with(field, Some(text))) {
            Err(GenError::Unread(lines)) => assert!(
                lines.len() == 1 && lines[0].contains(&format!("field `{field}`")),
                "the refusal of `{field}` with `{text}`: {lines:?}"
            ),
            other => panic!("`{field}` with `{text}` was not refused: {other:?}"),
        }
    }
    assert_eq!(generate(&layout_with("m1", None)), Ok(()), "`m1` unkeyed");
}

/// Generates the export decoder from `layout`, keeping only whether it was refused.
fn generated(layout: &Ledger) -> Result<(), GenError> {
    generate(layout, &[export::emit]).map(|_| ())
}

#[test]
fn export_index_base_missing_text_fails_generation() {
    check_refusal(generated);
}

negative_control!(
    export_index_base_missing_text_fails_generation,
    "a generator that ignores the entries' keys fails",
    expected = "`m1` with `body 1's mass` was not refused",
    check_refusal(|_| generated(&ledger::layout()))
);
