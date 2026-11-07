//! QA tests for TASK-M2-54, written from REQ-GEN-045's verify: `cargo xtask gate-provisional --milestone M2` no longer
//! lists `arctan_sum_bits`; control: an entry still marked provisional is listed. The run writes the report the gate
//! reads, so the test reads that file. Each test has a registered negative control (R-176).

// The file name `qa_TASK-M2-54` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use ledger::constants::{Citation, ConstantBuilder, ARCTAN_SUM_BITS, REGISTER};
use validation::negative_control;
use xtask::gate_report::{provisional, run_provisional};

/// The lines of `text` that list the entry.
fn listing(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|l| l.contains("arctan_sum_bits"))
        .collect()
}

/// M2's list over `register` holds no line for the entry.
fn check_not_listed(register: &[ConstantBuilder]) {
    let got = provisional("M2", register);
    let text = got.as_ref().unwrap_or_else(|t| t);
    assert_eq!(
        listing(text),
        Vec::<&str>::new(),
        "M2's list holds the entry"
    );
}

/// The register with the entry as it stood before R-699.
fn still_marked() -> Vec<ConstantBuilder> {
    REGISTER
        .iter()
        .map(|k| match k.name {
            "arctan_sum_bits" => ConstantBuilder {
                citation: Some(Citation::Calibration("REQ-GEN-044")),
                provisional: Some("M2"),
                ..ARCTAN_SUM_BITS
            },
            _ => *k,
        })
        .collect()
}

#[test]
fn qa_m2_54_m2_list_omits_the_entry() {
    check_not_listed(REGISTER);
    // The other M2 entries, and so the gate's own failure, are not this requirement's: the list is checked, not the exit.
    let root = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("qa_m2_54");
    let _ = std::fs::remove_dir_all(&root);
    let _ = run_provisional(&root, "M2");
    let report = root.join("target/gate-report/M2-provisional.txt");
    let text = std::fs::read_to_string(&report)
        .unwrap_or_else(|e| panic!("read {}: {e}", report.display()));
    assert_eq!(
        listing(&text),
        Vec::<&str>::new(),
        "the report holds the entry"
    );
}

negative_control!(
    qa_m2_54_m2_list_omits_the_entry,
    "an entry still marked provisional until M2 is listed (REQ-GEN-045's control)",
    expected = "M2's list holds the entry",
    check_not_listed(&still_marked())
);

/// The marked entry's line carries its gate and requirement: the listing above is the one that would name it.
fn check_named(register: &[ConstantBuilder]) {
    let got = provisional("M2", register);
    let text = got.expect_err("a marked entry fails the gate");
    let lines = listing(&text);
    assert_eq!(lines.len(), 1, "the marked entry's lines: {text}");
    assert!(
        lines[0].contains("until M2") && lines[0].contains("REQ-GEN-044"),
        "the line: {}",
        lines[0]
    );
}

#[test]
fn qa_m2_54_a_marked_entry_is_named_with_its_gate() {
    check_named(&still_marked());
}

negative_control!(
    qa_m2_54_a_marked_entry_is_named_with_its_gate,
    "the register as it stands has no such line",
    expected = "the marked entry's lines",
    check_named(REGISTER)
);
