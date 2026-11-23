//! The change-10 re-run (TASK-M3-36; R-35; REQ-VAL-036): the kernel's Aarseth–Zare against the patched NumPy
//! reference's recorded dumps, under `validation::change10`'s criteria, fixed before the first run: at most `1e-13` at
//! `t ≤ 2`, at most `1e-10` at `t = 13`, the same reference body at every sync boundary.
//!
//! Control: the kernel without the landing fix, the overshoot change 10 removed, fails them.

use std::path::{Path, PathBuf};

use validation::change10::{judge, table, text, Landing};
use validation::negative_control;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn check(landing: Landing) {
    let rows = table(&root(), landing).expect("the dumps and the fixture load");
    println!("{}", text(&rows, landing));
    let bad = judge(&rows);
    assert!(
        bad.is_empty(),
        "the change-10 re-run misses its criteria: {}",
        bad.join("; ")
    );
}

#[test]
fn change10_rerun_matches_the_patched_reference() {
    check(Landing::Fix);
}

negative_control!(
    change10_rerun_matches_the_patched_reference,
    "the kernel without the landing fix must miss the criteria against the patched reference",
    expected = "misses its criteria",
    check(Landing::Off)
);
