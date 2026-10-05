//! `RenderQuad`, the CPU-written quad record (dd_generation_root §3.7a; render contract Part 1): the generated layout
//! against the ledger table field by field, the ledger table against §3.7a's, and render contract Part 1's field list a
//! subset (REQ-PAY-090, R-256's applied note). Each test registers its negative control (R-176).

use ledger::payload;
use ledger::quad::{self, QuadField, RENDER_QUAD};
use ledger::schema::{Storage, Struct};
use validation::negative_control;

/// Render contract Part 1's `RenderQuad` field list, in its order.
const PART_1: [&str; 9] = [
    "quad_depth",
    "quad_state",
    "coherence_score",
    "outcome_impurity",
    "ensemble_spread",
    "suspect_fraction",
    "priority_score",
    "ancestor_gap",
    "cache_age",
];

/// A storage's name as §3.7a's type column and the WGSL struct write it.
fn type_name(storage: Storage) -> &'static str {
    match storage {
        Storage::U32 => "u32",
        Storage::F32 => "f32",
        _ => "not a 4-byte scalar",
    }
}

/// The `name: type` member lines of the one WGSL struct in `wgsl`.
fn members(wgsl: &str) -> Vec<(String, String)> {
    wgsl.lines()
        .skip_while(|l| !l.starts_with("struct "))
        .skip(1)
        .take_while(|l| !l.starts_with('}'))
        .filter_map(|l| {
            let (name, ty) = l.trim().trim_end_matches(',').split_once(": ")?;
            Some((name.to_owned(), ty.to_owned()))
        })
        .collect()
}

/// The generated struct `s` matches `table` field by field: the same names and types in the same order, in its WGSL
/// and in its layout, each member one 4-byte scalar at offset 4·k, 40 B in all with no padding.
fn check_generated(s: &Struct, table: &[QuadField]) {
    let want: Vec<(String, String)> = table
        .iter()
        .map(|f| (f.name.to_owned(), type_name(f.storage).to_owned()))
        .collect();
    assert_eq!(
        members(&quad::wgsl_struct(s)),
        want,
        "the generated RenderQuad differs from the ledger table"
    );
    let (offsets, size) = ledger::gen::rust::offsets(s);
    let packed: Vec<u32> = (0..table.len() as u32).map(|k| 4 * k).collect();
    assert_eq!(
        (offsets, size),
        (packed, 4 * table.len() as u32),
        "the generated RenderQuad differs from the ledger table: not packed 4-byte scalars"
    );
}

#[test]
fn renderquad_fields_generated_layout_matches_the_ledger_table() {
    check_generated(&quad::render_quad(), &RENDER_QUAD);
    assert_eq!(quad::render_quad().name, "RenderQuad");
}

negative_control!(
    renderquad_fields_generated_layout_matches_the_ledger_table,
    "a RenderQuad with cache_age and ancestor_gap swapped differs from the table",
    expected = "the generated RenderQuad differs from the ledger table",
    check_generated(
        &{
            let mut s = quad::render_quad();
            s.members.swap(7, 8);
            s
        },
        &RENDER_QUAD
    )
);

/// §3.7a's table in `doc`, the generation root's text: each row's member and type, in order.
fn section_3_7a(doc: &str) -> Vec<(String, String)> {
    doc.lines()
        .skip_while(|l| !l.starts_with("### 3.7a "))
        .skip(1)
        .take_while(|l| !l.starts_with("### "))
        .filter_map(|l| {
            let cells: Vec<&str> = l.split('|').map(str::trim).collect();
            let name = cells.get(1)?.strip_prefix('`')?.strip_suffix('`')?;
            Some((name.to_owned(), (*cells.get(2)?).to_owned()))
        })
        .collect()
}

fn generation_root() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/design/principia_dd_generation_root.md"
    ))
    .expect("read dd_generation_root")
}

/// The ledger table is §3.7a's, row by row.
fn check_transcribed(doc: &str, table: &[QuadField]) {
    let want: Vec<(String, String)> = table
        .iter()
        .map(|f| (f.name.to_owned(), type_name(f.storage).to_owned()))
        .collect();
    assert_eq!(
        section_3_7a(doc),
        want,
        "the ledger's RenderQuad table differs from dd_generation_root §3.7a"
    );
}

#[test]
fn renderquad_fields_ledger_table_is_section_3_7a() {
    check_transcribed(&generation_root(), &RENDER_QUAD);
}

negative_control!(
    renderquad_fields_ledger_table_is_section_3_7a,
    "a doc whose cache_age is f32 differs from the ledger table",
    expected = "the ledger's RenderQuad table differs from dd_generation_root §3.7a",
    check_transcribed(
        &generation_root().replace("| `cache_age` | u32 |", "| `cache_age` | f32 |"),
        &RENDER_QUAD
    )
);

/// §3.7a's `quad_state` codes in `doc`: `0 loaded · 1 pending · …` in the row's note.
fn check_states(doc: &str, states: &[&str]) {
    let row = doc
        .lines()
        .find(|l| l.starts_with("| `quad_state` |"))
        .expect("§3.7a's quad_state row");
    let listed: String = states
        .iter()
        .enumerate()
        .map(|(k, s)| format!("{k} {s}"))
        .collect::<Vec<_>>()
        .join(" · ");
    assert!(
        row.contains(&listed),
        "the quad_state codes differ from §3.7a: {listed:?} is not in {row:?}"
    );
}

#[test]
fn renderquad_fields_quad_state_codes_are_section_3_7a() {
    check_states(&generation_root(), &quad::quad_states());
}

negative_control!(
    renderquad_fields_quad_state_codes_are_section_3_7a,
    "codes with pending and loaded swapped differ from §3.7a",
    expected = "the quad_state codes differ from §3.7a",
    check_states(
        &generation_root(),
        &["pending", "loaded", "refinable", "terminal", "stale"]
    )
);

/// Each of `names` is a member of the ledger table.
fn check_subset(names: &[&str]) {
    let missing: Vec<&&str> = names
        .iter()
        .filter(|n| !RENDER_QUAD.iter().any(|f| f.name == **n))
        .collect();
    assert!(
        missing.is_empty(),
        "render contract Part 1's RenderQuad names absent from the ledger table: {missing:?}"
    );
}

#[test]
fn renderquad_fields_render_contract_part_1_is_a_subset() {
    check_subset(&PART_1);
}

negative_control!(
    renderquad_fields_render_contract_part_1_is_a_subset,
    "majority_class is no RenderQuad field (R-20), so the subset check must fail on it",
    expected = "render contract Part 1's RenderQuad names absent from the ledger table",
    check_subset(&["majority_class"])
);

/// No payload struct holds `name`, so the schema version's hash, over the payload structs, never sees it (§3.7a).
fn check_not_payload(name: &str) {
    assert!(
        payload::structs().iter().all(|s| s.name != name),
        "`{name}` is a payload struct, so the schema version hashes it"
    );
}

#[test]
fn renderquad_fields_not_in_the_payload() {
    check_not_payload(quad::render_quad().name);
}

negative_control!(
    renderquad_fields_not_in_the_payload,
    "ICDescriptor is a payload struct",
    expected = "is a payload struct, so the schema version hashes it",
    check_not_payload("ICDescriptor")
);
