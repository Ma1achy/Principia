//! QA tests for TASK-M2-54, written from REQ-GEN-045, R-699 and dd_generation_root §3.8, not from the implementation.
//! - REQ-GEN-045 (`qa_m2_54_entry_*`): the register holds one `arctan_sum_bits`, a confirmed entry: 100 exactly, unmarked,
//!   citing R-699's own heading in `decisions.md`, the class R-699 lets stand (`canonical-units`), and it passes the
//!   metadata gate; no gate's list holds it; `sum_bits()` reads 100.
//! - REQ-GEN-045, "the generated `arctan.rs` unchanged" (`qa_m2_54_generated_*`): the emitter's one file and the
//!   checked-in file both hash to [`ARCTAN_FNV`], the FNV-1a of the file as it stood before this task.
//! - R-699, "Not hashed into the schema version" (`qa_m2_54_unhashed`): the register without the entry, or with another
//!   value, has the same version.
//! - dd_generation_root §3.8, the deliverable (`qa_m2_54_doc_*`): one row, four columns, the confirmed table's, citing
//!   R-699, and R-699's dated note beside it; the provisional table's five columns no longer hold it.
//!
//! Each test has a registered negative control (R-176).

// The file name `qa_TASK-M2-54` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use ledger::constants::{
    gate, provisional_until, Admissibility, Citation, ConstantBuilder, Value, ARCTAN_SUM_BITS,
    MILESTONES, REGISTER,
};
use ledger::gen::arctan::{arctan_file, emit, sum_bits, ARCTAN_PATH};
use ledger::version::{schema_version, Hashed};
use validation::negative_control;

/// The start of R-699's heading in `decisions.md`.
const R699: &str = "## R-699 — ";

/// FNV-1a (64-bit) of `crates/kernel/src/generated/arctan.rs` at this task's merge base, a0225c68, 1217 bytes, taken from
/// that commit's blob with git and Python, never from this task's emitter. REQ-GEN-045: the file is unchanged.
const ARCTAN_FNV: u64 = 0x551d_c88b_00f9_54ea;

fn root_file(path: &str) -> String {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path);
    std::fs::read_to_string(&p)
        .unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
        .replace("\r\n", "\n")
}

fn fnv1a(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// The only line of `decisions.md` that heads R-699, without its `## `.
fn r699_heading(decisions: &str) -> &str {
    let mut heads = decisions.lines().filter(|l| l.starts_with(R699));
    let head = heads.next().expect("decisions.md has no R-699 heading");
    assert!(heads.next().is_none(), "decisions.md heads R-699 twice");
    &head[3..]
}

// ── REQ-GEN-045: the entry ───────────────────────────────────────────────────────────────────────────────────────

/// `k` stands as REQ-GEN-045 and R-699 say.
fn check_entry(k: ConstantBuilder) {
    assert_eq!(k.value, Some(Value::Exact(100.0)), "the entry's value");
    assert_eq!(k.provisional, None, "the entry is marked provisional");
    assert_eq!(
        k.class,
        Some(Admissibility::CanonicalUnits),
        "the entry's class is not the one R-699 lets stand"
    );
    let decisions = root_file("decisions.md");
    let cited = match k.citation {
        Some(Citation::Corpus {
            file: "decisions.md",
            section,
        }) => section,
        other => panic!("the entry does not cite R-699's heading: {other:?}"),
    };
    assert_eq!(
        cited,
        r699_heading(&decisions),
        "the entry does not cite R-699's heading"
    );
    k.build()
        .unwrap_or_else(|e| panic!("the entry is refused: {e}"));
}

/// The register's one `arctan_sum_bits`.
fn the_entry(register: &[ConstantBuilder]) -> ConstantBuilder {
    let held: Vec<_> = register
        .iter()
        .filter(|k| k.name == "arctan_sum_bits")
        .collect();
    assert_eq!(held.len(), 1, "the register's `arctan_sum_bits` entries");
    *held[0]
}

#[test]
fn qa_m2_54_entry_is_confirmed() {
    check_entry(the_entry(REGISTER));
    assert_eq!(sum_bits(), 100, "`sum_bits()` does not read 100");
    gate(REGISTER).unwrap_or_else(|e| panic!("the register is refused: {e:?}"));
}

negative_control!(
    qa_m2_54_entry_is_confirmed,
    "the entry as it stood before R-699: its mark and its calibration requirement",
    expected = "the entry is marked provisional",
    check_entry(ConstantBuilder {
        citation: Some(Citation::Calibration("REQ-GEN-044")),
        provisional: Some("M2"),
        ..the_entry(REGISTER)
    })
);

mod marked_only {
    use super::*;
    negative_control!(
        qa_m2_54_entry_is_confirmed,
        "the mark off but the citation still the calibration requirement",
        expected = "does not cite R-699's heading",
        check_entry(ConstantBuilder {
            citation: Some(Citation::Calibration("REQ-GEN-044")),
            ..the_entry(REGISTER)
        })
    );
}

mod another_value {
    use super::*;
    negative_control!(
        qa_m2_54_entry_is_confirmed,
        "101 writes the same table (the plateau) but is not the value R-699 confirms",
        expected = "the entry's value",
        check_entry(ConstantBuilder {
            value: Some(Value::Exact(101.0)),
            ..the_entry(REGISTER)
        })
    );
}

mod another_class {
    use super::*;
    negative_control!(
        qa_m2_54_entry_is_confirmed,
        "a class other than the one R-699 lets stand",
        expected = "the entry's class",
        check_entry(ConstantBuilder {
            class: Some(Admissibility::AchievableMaximum),
            ..the_entry(REGISTER)
        })
    );
}

mod another_heading {
    use super::*;
    negative_control!(
        qa_m2_54_entry_is_confirmed,
        "R-699 in a section text that is not its heading",
        expected = "does not cite R-699's heading",
        check_entry(ConstantBuilder {
            citation: Some(Citation::Corpus {
                file: "decisions.md",
                section: "R-699 — `arctan_sum_bits` = 100 is confirmed",
            }),
            ..the_entry(REGISTER)
        })
    );
}

mod duplicated {
    use super::*;
    negative_control!(
        qa_m2_54_entry_is_confirmed,
        "a register holding the entry twice",
        expected = "entries",
        {
            the_entry(&[ARCTAN_SUM_BITS, ARCTAN_SUM_BITS]);
        }
    );
}

/// No gate's list over `register` holds the entry.
fn check_on_no_list(register: &[ConstantBuilder]) {
    for m in MILESTONES {
        assert!(
            !provisional_until(register, m).contains(&"arctan_sum_bits"),
            "the entry is on {m}'s list"
        );
    }
}

#[test]
fn qa_m2_54_entry_is_on_no_list() {
    check_on_no_list(REGISTER);
}

negative_control!(
    qa_m2_54_entry_is_on_no_list,
    "the entry marked until M2 is on M2's list",
    expected = "the entry is on M2's list",
    check_on_no_list(&[ConstantBuilder {
        citation: Some(Citation::Calibration("REQ-GEN-044")),
        provisional: Some("M2"),
        ..ARCTAN_SUM_BITS
    }])
);

// ── REQ-GEN-045: the generated file ──────────────────────────────────────────────────────────────────────────────

/// `text` is the file as it stood before this task.
fn check_unchanged(text: &str) {
    assert_eq!(fnv1a(text), ARCTAN_FNV, "the file changed");
}

#[test]
fn qa_m2_54_generated_file_is_unchanged() {
    let files = emit(&[], &[]);
    assert_eq!(files.len(), 1, "the emitter's files");
    assert_eq!(files[0].path.to_str(), Some(ARCTAN_PATH), "the file's path");
    check_unchanged(&files[0].contents);
    check_unchanged(&root_file(ARCTAN_PATH));
}

negative_control!(
    qa_m2_54_generated_file_is_unchanged,
    "one hex digit of the file changed",
    expected = "the file changed",
    check_unchanged(&arctan_file().replacen("0x1FD5_BA9A", "0x1FD5_BA9B", 1))
);

// ── R-699: not hashed ────────────────────────────────────────────────────────────────────────────────────────────

fn version_with(register: &[ConstantBuilder]) -> u64 {
    let l = ledger::layout();
    let entries = ledger::gen::validate(&l).unwrap_or_else(|e| panic!("{e}"));
    let structs = ledger::payload::structs();
    let h = Hashed {
        register,
        ..Hashed::payload(&l.words, &entries, &structs)
    };
    schema_version(&h).unwrap_or_else(|e| panic!("{e}"))
}

/// The version is unchanged by giving the entry `name` another value.
fn check_value_unhashed(name: &str) {
    let mut other = REGISTER.to_vec();
    let k = other
        .iter_mut()
        .find(|k| k.name == name)
        .unwrap_or_else(|| panic!("the register holds no `{name}`"));
    k.value = Some(Value::Exact(2.0 * k.number() + 1.0));
    assert_eq!(
        version_with(&other),
        version_with(REGISTER),
        "changing `{name}`'s value changed the version"
    );
}

#[test]
fn qa_m2_54_unhashed() {
    check_value_unhashed("arctan_sum_bits");
    let without: Vec<_> = REGISTER
        .iter()
        .filter(|k| k.name != "arctan_sum_bits")
        .copied()
        .collect();
    assert_eq!(
        version_with(&without),
        version_with(REGISTER),
        "removing the entry changed the version"
    );
}

negative_control!(
    qa_m2_54_unhashed,
    "f16_finite_max decides stored bits and is hashed",
    expected = "changed the version",
    check_value_unhashed("f16_finite_max")
);

// ── dd_generation_root §3.8 ──────────────────────────────────────────────────────────────────────────────────────

/// The table cells of `row`, trimmed.
fn cells(row: &str) -> Vec<&str> {
    let inner = row.trim().trim_start_matches('|').trim_end_matches('|');
    inner.split(" | ").map(str::trim).collect()
}

/// `doc` holds the entry's one row, in the confirmed table's four columns (name, value, class, citation), and R-699's
/// dated note after it.
fn check_doc(doc: &str) {
    let rows: Vec<&str> = doc
        .lines()
        .filter(|l| l.starts_with("| `arctan_sum_bits` |"))
        .collect();
    assert_eq!(rows.len(), 1, "§3.8's rows for the entry");
    let row = cells(rows[0]);
    assert_eq!(
        row.len(),
        4,
        "the row is not the confirmed table's: {row:?}"
    );
    assert!(row[1].starts_with("100,"), "the row's value: {}", row[1]);
    assert_eq!(row[2], "canonical-units", "the row's class");
    assert_eq!(row[3], "R-699", "the row's citation");
    let at = doc.find(rows[0]).expect("the row is in the text");
    assert!(
        doc[at..].contains("*4 Nov 2026 (R-699, closing RQ-635):* `arctan_sum_bits`"),
        "R-699's dated note is not after the row"
    );
}

#[test]
fn qa_m2_54_doc_row_is_in_the_confirmed_table() {
    check_doc(&root_file("docs/design/principia_dd_generation_root.md"));
}

negative_control!(
    qa_m2_54_doc_row_is_in_the_confirmed_table,
    "the provisional table's row, as it stood before this task",
    expected = "the row is not the confirmed table's",
    check_doc(
        "| `arctan_sum_bits` | 100, a working precision | canonical-units | M2 | REQ-GEN-044 |\n\n\
         *4 Nov 2026 (R-699, closing RQ-635):* `arctan_sum_bits` above"
    )
);

mod no_note {
    use super::*;
    negative_control!(
        qa_m2_54_doc_row_is_in_the_confirmed_table,
        "the row moved, its dated note dropped",
        expected = "R-699's dated note",
        check_doc("| `arctan_sum_bits` | 100, a working precision | canonical-units | R-699 |\n")
    );
}

mod both_tables {
    use super::*;
    negative_control!(
        qa_m2_54_doc_row_is_in_the_confirmed_table,
        "the row added to the confirmed table and left in the provisional one",
        expected = "§3.8's rows",
        check_doc(
            "| `arctan_sum_bits` | 100, | canonical-units | M2 | REQ-GEN-044 |\n\
             | `arctan_sum_bits` | 100, | canonical-units | R-699 |\n"
        )
    );
}
