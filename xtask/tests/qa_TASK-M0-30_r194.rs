//! QA tests for TASK-M0-30, written from REQ-VAL-161 — "the helpers and inline checks in qa's
//! `xtask/tests/qa_TASK-M0-01_r191.rs`, `_r193.rs` and `_r194.rs` must live in shared test-support modules that each
//! test and a control can both call, so a control in a new target trips the test's own assertion and shares its
//! inputs" — not from the implementation.
//!
//! This file is such a new target for `support/qa_m0_01_r194.rs`: it uses every item of the module, including the
//! shared feature tables `A_DEFAULT_B_NOT` and `X_NOT_DEFAULT` (R-218), and runs each check on an input it must accept
//! (the test) and on one it must reject, tripping the check's own assertion by its message (the control, R-176,
//! R-212). Workspaces are named `qa30_*`, apart from the cases of `qa_TASK-M0-01_r194.rs`, which write to the same
//! directory.
// The file name `qa_TASK-M0-30_…` gives a crate name that is not snake case.
#![allow(non_snake_case)]

#[path = "support/qa_m0_01_r194.rs"]
mod qa_m0_01_r194;

use qa_m0_01_r194::*;
use std::path::{Path, PathBuf};
use validation::negative_control;

/// A workspace whose `krate`, with the validation dev-dependency and `features`, has `lib`.
fn with_lib(case: &str, krate: &str, features: &str, lib: String) -> PathBuf {
    workspace(
        case,
        &[
            (
                format!("crates/{krate}/Cargo.toml"),
                crate_manifest(krate, features),
            ),
            (format!("crates/{krate}/src/lib.rs"), lib),
        ],
    )
}

/// A ledger whose doctest uses validation.
const DOC_USE: &str = "/// ```\n/// let _ = validation::Harness;\n/// ```\npub fn f() {}\n";

/// `fails_and_its_control_passes`, called from a new target, on a ledger unit test under non-default `x` that only
/// `--all-features` builds (dev profile), with the default cell as the other.
#[test]
fn qa_m0_30_r194_case_and_control_accept_an_all_features_cell() {
    fails_and_its_control_passes(
        "qa30_all_dev_ledger",
        "ledger",
        X_NOT_DEFAULT,
        "all(test, feature = \"x\")",
        &["--all-features"],
        &[&[]],
    );
}

negative_control!(
    qa_m0_30_r194_case_and_control_accept_an_all_features_cell,
    "`--features x` builds the unit test too, so the isolation premise must reject it",
    expected = "builds the unit test too, so the case does not isolate",
    fails_and_its_control_passes(
        "qa30_all_dev_ledger_nc",
        "ledger",
        X_NOT_DEFAULT,
        "all(test, feature = \"x\")",
        &["--all-features"],
        &[&["--features", "x"]],
    )
);

/// `premise_the_cell_runs_the_unit_test` accepts the default-features cell of a unit test under `a` on, `b` off.
#[test]
fn qa_m0_30_r194_cell_premise_accepts_the_cell_that_builds_the_test() {
    let root = with_lib(
        "qa30_cell",
        "kernel",
        A_DEFAULT_B_NOT,
        lib_rs("all(test, feature = \"a\", not(feature = \"b\"))", true),
    );
    premise_the_cell_runs_the_unit_test("qa30_cell", &root, "kernel", &["--lib"]);
}

negative_control!(
    qa_m0_30_r194_cell_premise_accepts_the_cell_that_builds_the_test,
    "`--features b` does not build the unit test, so the cell premise must reject it",
    expected = "does not run the unit test that uses validation",
    premise_the_cell_runs_the_unit_test(
        "qa30_cell_nc",
        &with_lib(
            "qa30_cell_nc",
            "kernel",
            A_DEFAULT_B_NOT,
            lib_rs("all(test, feature = \"a\", not(feature = \"b\"))", true),
        ),
        "kernel",
        &["--lib", "--features", "b"],
    )
);

/// `premise_no_other_cell_builds_it` accepts a cell that does not build the unit test.
#[test]
fn qa_m0_30_r194_isolation_premise_accepts_a_cell_that_does_not_build_it() {
    let root = with_lib(
        "qa30_iso",
        "kernel",
        X_NOT_DEFAULT,
        lib_rs("all(test, feature = \"x\")", true),
    );
    premise_no_other_cell_builds_it("qa30_iso", &root, "kernel", &[&[]]);
}

negative_control!(
    qa_m0_30_r194_isolation_premise_accepts_a_cell_that_does_not_build_it,
    "`--all-features` builds the unit test, so the isolation premise must reject it",
    expected = "builds the unit test too, so the case does not isolate",
    premise_no_other_cell_builds_it(
        "qa30_iso_nc",
        &with_lib(
            "qa30_iso_nc",
            "kernel",
            X_NOT_DEFAULT,
            lib_rs("all(test, feature = \"x\")", true),
        ),
        "kernel",
        &[&["--all-features"]],
    )
);

/// `check_the_case_fails` accepts a kernel unit test that uses validation.
#[test]
fn qa_m0_30_r194_case_check_accepts_a_unit_test_use() {
    let root = with_lib("qa30_case", "kernel", "", lib_rs("test", true));
    check_the_case_fails(
        "qa30_case",
        &root,
        "kernel",
        "crates/kernel/src/lib.rs",
        &["--lib"],
    );
}

negative_control!(
    qa_m0_30_r194_case_check_accepts_a_unit_test_use,
    "the same workspace without the use passes xtask deps, so the case check must reject it",
    expected = "builds with validation passes xtask deps",
    check_the_case_fails(
        "qa30_case_nc",
        &with_lib("qa30_case_nc", "kernel", "", lib_rs("test", false)),
        "kernel",
        "crates/kernel/src/lib.rs",
        &["--lib"],
    )
);

/// `check_the_control_passes` accepts the workspace without the use.
#[test]
fn qa_m0_30_r194_control_check_accepts_the_workspace_without_the_use() {
    let root = with_lib("qa30_control", "kernel", "", lib_rs("test", false));
    check_the_control_passes("qa30_control", &root);
}

negative_control!(
    qa_m0_30_r194_control_check_accepts_the_workspace_without_the_use,
    "a workspace with the use fails xtask deps, so the control check must reject it",
    expected = "the same workspace without the use fails xtask deps",
    check_the_control_passes(
        "qa30_control_nc",
        &with_lib("qa30_control_nc", "kernel", "", lib_rs("test", true)),
    )
);

/// `premise_the_doctest_runs` and `check_the_doctest_passes` accept a ledger doctest that uses validation.
#[test]
fn qa_m0_30_r194_doctest_checks_accept_a_doctest_use() {
    let root = with_lib("qa30_doc", "ledger", "", DOC_USE.to_owned());
    premise_the_doctest_runs(&root);
    check_the_doctest_passes(&root);
}

negative_control!(
    qa_m0_30_r194_doctest_checks_accept_a_doctest_use,
    "the same use in a ledger unit test fails xtask deps, so the doctest check must reject it",
    expected = "a ledger doctest that uses validation fails xtask deps",
    check_the_doctest_passes(&with_lib("qa30_doc_nc", "ledger", "", lib_rs("test", true),))
);

/// `check_the_unit_test_use_fails` accepts a ledger unit test that uses validation.
#[test]
fn qa_m0_30_r194_unit_test_use_check_accepts_a_unit_test_use() {
    let root = with_lib("qa30_unit", "ledger", "", lib_rs("test", true));
    check_the_unit_test_use_fails(&root);
}

negative_control!(
    qa_m0_30_r194_unit_test_use_check_accepts_a_unit_test_use,
    "a doctest use passes xtask deps, so the unit-test check must reject it",
    expected = "the same use in a ledger unit test passes xtask deps",
    check_the_unit_test_use_fails(&with_lib("qa30_unit_nc", "ledger", "", DOC_USE.to_owned()))
);

/// `qa_m0_30_r194_runners_report_a_passing_doctest`'s doctest check, called by the test and its control: `cargo test
/// --doc` on `root`'s ledger passes and reports one doctest passed.
fn check_the_doctest_count(root: &Path) {
    let (ok, out) = cargo_test(root, "ledger", &["--doc"]);
    assert!(ok && out.contains("1 passed"), "cargo test --doc: {out}");
}

/// `cargo_test` and `deps`, called directly: the doctest premise's runner reports the doctest's pass, and `xtask deps`
/// passes the doctest workspace.
#[test]
fn qa_m0_30_r194_runners_report_a_passing_doctest() {
    let root = with_lib("qa30_runners", "ledger", "", DOC_USE.to_owned());
    check_the_doctest_count(&root);
    let (ok, stdout, stderr) = deps(&root);
    assert!(
        ok,
        "xtask deps on the doctest workspace fails:\n{stdout}\n{stderr}"
    );
}

negative_control!(
    qa_m0_30_r194_runners_report_a_passing_doctest,
    "a ledger without the doctest runs none, so the doctest-count check must reject it",
    expected = "cargo test --doc:",
    {
        let root = with_lib(
            "qa30_runners_nc",
            "ledger",
            "",
            "pub fn f() {}\n".to_owned(),
        );
        check_the_doctest_count(&root);
    }
);
