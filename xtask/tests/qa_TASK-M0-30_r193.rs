//! QA tests for TASK-M0-30, written from REQ-VAL-161 — "the helpers and inline checks in qa's
//! `xtask/tests/qa_TASK-M0-01_r191.rs`, `_r193.rs` and `_r194.rs` must live in shared test-support modules that each
//! test and a control can both call, so a control in a new target trips the test's own assertion and shares its
//! inputs" — not from the implementation.
//!
//! This file is such a new target for `support/qa_m0_01_r193.rs`: it uses every item of the module, and runs each
//! check on an input it must accept (the test) and on one it must reject, tripping the check's own assertion by its
//! message (the control, R-176, R-212). Workspaces are named `qa30_*`, apart from the cases of
//! `qa_TASK-M0-01_r193.rs`, which write to the same directory.
#![allow(non_snake_case)]

#[path = "support/qa_m0_01_r193.rs"]
mod qa_m0_01_r193;

use qa_m0_01_r193::*;
use std::path::PathBuf;
use validation::negative_control;

/// A kernel workspace, with the validation dev-dependency, whose unit test under `cfg(test)` uses validation or not.
fn kernel(case: &str, used: bool) -> PathBuf {
    workspace(
        case,
        &[
            (
                "crates/kernel/Cargo.toml".to_owned(),
                crate_manifest("kernel", "", ""),
            ),
            ("crates/kernel/src/lib.rs".to_owned(), lib_rs("test", used)),
        ],
    )
}

/// `fails_and_its_control_passes`, called from a new target, on a plain `cfg(test)` unit test of ledger.
#[test]
fn qa_m0_30_r193_case_and_control_accept_a_plain_unit_test_use() {
    fails_and_its_control_passes("qa30_plain_ledger", "ledger", "", "", "test", &[]);
}

negative_control!(
    qa_m0_30_r193_case_and_control_accept_a_plain_unit_test_use,
    "`cfg(any())` is never built by cargo test, so the premise must reject it",
    expected = "does not run the unit test that uses validation",
    fails_and_its_control_passes("qa30_plain_ledger_nc", "ledger", "", "", "any()", &[])
);

/// `check_the_case_fails` accepts a kernel unit test that uses validation (premise first, as the case runs it).
#[test]
fn qa_m0_30_r193_case_check_accepts_a_unit_test_use() {
    let root = kernel("qa30_case", true);
    premise_cargo_test_runs_the_unit_test("qa30_case", &root, "kernel", &[]);
    check_the_case_fails(
        "qa30_case",
        &root,
        "kernel",
        "crates/kernel/src/lib.rs",
        &[],
    );
}

negative_control!(
    qa_m0_30_r193_case_check_accepts_a_unit_test_use,
    "the same workspace without the use passes xtask deps, so the case check must reject it",
    expected = "builds with validation passes xtask deps",
    check_the_case_fails(
        "qa30_case_nc",
        &kernel("qa30_case_nc", false),
        "kernel",
        "crates/kernel/src/lib.rs",
        &[],
    )
);

/// `check_the_control_passes` accepts the same kernel workspace without the use.
#[test]
fn qa_m0_30_r193_control_check_accepts_the_workspace_without_the_use() {
    check_the_control_passes("qa30_control", &kernel("qa30_control", false));
}

negative_control!(
    qa_m0_30_r193_control_check_accepts_the_workspace_without_the_use,
    "a workspace with the use fails xtask deps, so the control check must reject it",
    expected = "the same workspace without the use fails xtask deps",
    check_the_control_passes("qa30_control_nc", &kernel("qa30_control_nc", true))
);

/// `deps` and `manifest`, called directly: `xtask deps` passes on a workspace whose kernel has no dev-dependency.
#[test]
fn qa_m0_30_r193_deps_passes_without_a_dev_dependency() {
    let root = workspace(
        "qa30_deps",
        &[(
            "crates/kernel/Cargo.toml".to_owned(),
            manifest(
                "",
                "kernel",
                "\n[build-dependencies]\nledger = { path = \"../ledger\" }\n",
            ),
        )],
    );
    let (ok, stdout, stderr) = deps(&root);
    assert!(
        ok,
        "xtask deps fails on a workspace without the dev-dependency:\n{stdout}\n{stderr}"
    );
}

negative_control!(
    qa_m0_30_r193_deps_passes_without_a_dev_dependency,
    "a unit test that uses validation fails xtask deps, so `deps` must report failure",
    expected = "xtask deps fails on",
    {
        let root = kernel("qa30_deps_nc", true);
        let (ok, stdout, stderr) = deps(&root);
        assert!(
            ok,
            "xtask deps fails on {}:\n{stdout}\n{stderr}",
            root.display()
        );
    }
);
