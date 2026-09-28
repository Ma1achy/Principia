//! QA tests for TASK-M0-30, written from REQ-VAL-161 — "the helpers and inline checks in qa's
//! `xtask/tests/qa_TASK-M0-01_r191.rs` ... must live in shared test-support modules that each test and a control can
//! both call, so a control in a new target trips the test's own assertion and shares its inputs" — not from the
//! implementation.
//!
//! This file is such a new target: it includes `support/qa_m0_01_r191.rs` as a control of TASK-M0-22 will, uses
//! every item of it (so the module compiles, without dead code, in a target other than the one it was moved from),
//! and runs each check twice: on an input it must accept (the test), and on one it must reject, tripping the check's
//! own assertion by its message (the control, R-176, R-212). Workspaces are named `qa30_*`, apart from the cases of
//! `qa_TASK-M0-01_r191.rs`, which write to the same directory.
// The file name `qa_TASK-M0-30_…` gives a crate name that is not snake case.
#![allow(non_snake_case)]

#[path = "support/qa_m0_01_r191.rs"]
mod qa_m0_01_r191;

use qa_m0_01_r191::*;
use std::path::PathBuf;
use validation::negative_control;

/// A scratch directory of this file's own, emptied.
fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("qa_TASK-M0-30_r191")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A kernel workspace whose unit test uses validation, with the dev-dependency (R-187 forbids it).
fn kernel_unit_use(case: &str) -> PathBuf {
    let cargo = kernel_manifest(VALIDATION_DEV);
    workspace(
        case,
        &[
            ("crates/kernel/Cargo.toml", &cargo),
            ("crates/kernel/src/lib.rs", UNIT_USE),
        ],
    )
}

/// A kernel workspace whose integration test uses validation, with the dev-dependency (R-187 allows it).
fn kernel_external_use(case: &str) -> PathBuf {
    let cargo = kernel_manifest(VALIDATION_DEV);
    workspace(
        case,
        &[
            ("crates/kernel/Cargo.toml", &cargo),
            ("crates/kernel/tests/uses.rs", EXTERNAL_USE),
        ],
    )
}

/// `fails_with_compiler_error`, called from a new target, accepts a kernel unit test that uses validation.
#[test]
fn qa_m0_30_r191_compiler_error_check_accepts_a_unit_test_use() {
    let root = kernel_unit_use("qa30_unit");
    let stderr =
        fails_with_compiler_error("qa30_unit", &root, "kernel", "crates/kernel/src/lib.rs");
    assert!(stderr.contains("error[E"), "{stderr}");
}

negative_control!(
    qa_m0_30_r191_compiler_error_check_accepts_a_unit_test_use,
    "the use in an integration test passes xtask deps, so the failure check must reject it",
    expected = "a unit test of kernel that uses validation passes xtask deps",
    {
        fails_with_compiler_error(
            "qa30_unit_nc",
            &kernel_external_use("qa30_unit_nc"),
            "kernel",
            "crates/kernel/tests/uses.rs",
        );
    }
);

/// `passes_the_compile_check` and `compiles_with_the_dependency`, called from a new target, accept a ledger
/// integration test that uses validation.
#[test]
fn qa_m0_30_r191_pass_check_accepts_an_integration_test_use() {
    let cargo = manifest("ledger", VALIDATION_DEV);
    let root = workspace(
        "qa30_ledger_it",
        &[
            ("crates/ledger/Cargo.toml", &cargo),
            ("crates/ledger/tests/uses.rs", EXTERNAL_USE),
        ],
    );
    passes_the_compile_check("qa30_ledger_it", &root);
    compiles_with_the_dependency("qa30_ledger_it", &root, "ledger");
}

negative_control!(
    qa_m0_30_r191_pass_check_accepts_an_integration_test_use,
    "a unit-test use fails xtask deps, so the pass check must reject it",
    expected = "xtask deps fails",
    passes_the_compile_check("qa30_ledger_it_nc", &kernel_unit_use("qa30_ledger_it_nc"))
);

/// `compiles_with_the_dependency` accepts a kernel whose integration test uses its validation dev-dependency.
#[test]
fn qa_m0_30_r191_compile_control_accepts_a_real_dependency() {
    let root = kernel_external_use("qa30_compiles");
    compiles_with_the_dependency("qa30_compiles", &root, "kernel");
}

negative_control!(
    qa_m0_30_r191_compile_control_accepts_a_real_dependency,
    "without the dev-dependency the use does not compile, so the compile control must reject it",
    expected = "does not compile even with its validation dev-dependency",
    compiles_with_the_dependency(
        "qa30_compiles_nc",
        &workspace(
            "qa30_compiles_nc",
            &[("crates/kernel/tests/uses.rs", EXTERNAL_USE)],
        ),
        "kernel",
    )
);

/// `premise_cargo_test_lib_runs_the_unit_test` and `check_lib_test_false_fails`, called from a new target, on a lib
/// with `test = false` whose unit test uses validation.
#[test]
fn qa_m0_30_r191_lib_test_false_check_accepts_the_use() {
    let cargo = kernel_manifest(&format!("\n[lib]\ntest = false\n{VALIDATION_DEV}"));
    let root = workspace(
        "qa30_test_false",
        &[
            ("crates/kernel/Cargo.toml", &cargo),
            ("crates/kernel/src/lib.rs", UNIT_USE),
        ],
    );
    premise_cargo_test_lib_runs_the_unit_test(&root);
    check_lib_test_false_fails(&root);
}

negative_control!(
    qa_m0_30_r191_lib_test_false_check_accepts_the_use,
    "the same lib without the use passes xtask deps, so the test = false check must reject it",
    expected = "in a lib with `test = false`, passes xtask deps",
    check_lib_test_false_fails(&workspace(
        "qa30_test_false_nc",
        &[
            (
                "crates/kernel/Cargo.toml",
                &kernel_manifest(&format!("\n[lib]\ntest = false\n{VALIDATION_DEV}")),
            ),
            ("crates/kernel/src/lib.rs", "pub fn f() {}\n"),
        ],
    ))
);

/// The unchanged-workspace and stable-target-directory checks, called from a new target, in the order
/// `qa_the_check_leaves_the_workspace_unchanged_and_uses_a_stable_target_dir_under_target` runs them.
#[test]
fn qa_m0_30_r191_unchanged_and_stable_target_checks_accept_the_check() {
    let root = kernel_unit_use("qa30_unchanged");
    let target = target_directory(&root);
    let before = dirs(&target);
    let (manifest_before, lib_before) = (
        read(&root, "crates/kernel/Cargo.toml"),
        read(&root, "crates/kernel/src/lib.rs"),
    );
    check_the_use_fails(&root);
    check_unchanged(&root, "crates/kernel/Cargo.toml", &manifest_before);
    check_unchanged(&root, "crates/kernel/src/lib.rs", &lib_before);
    check_no_orig(&root);
    let after_first = dirs(&target);
    check_a_new_target_dir(&target, &before, &after_first);
    std::fs::write(root.join("crates/kernel/src/lib.rs"), "pub fn f() {}\n").unwrap();
    passes_the_compile_check("qa30_unchanged_second", &root);
    check_the_same_target_dir(&target, &after_first);
}

negative_control!(
    qa_m0_30_r191_unchanged_and_stable_target_checks_accept_the_check,
    "a workspace without the use passes xtask deps, so the use check must reject it",
    expected = "the unit test that uses validation passes",
    check_the_use_fails(&kernel_external_use("qa30_unchanged_nc"))
);

/// `check_unchanged` accepts a file whose bytes are those read before.
#[test]
fn qa_m0_30_r191_unchanged_check_accepts_the_same_bytes() {
    let root = kernel_unit_use("qa30_bytes");
    let before = read(&root, "crates/kernel/src/lib.rs");
    check_unchanged(&root, "crates/kernel/src/lib.rs", &before);
}

negative_control!(
    qa_m0_30_r191_unchanged_check_accepts_the_same_bytes,
    "a file whose bytes changed must fail the unchanged check, naming the file",
    expected = "xtask deps changed kernel's src/lib.rs",
    check_unchanged(
        &kernel_unit_use("qa30_bytes_nc"),
        "crates/kernel/src/lib.rs",
        b"pub fn f() {}\n",
    )
);

/// `check_no_orig` accepts a workspace with no `Cargo.toml.orig` beside kernel's manifest.
#[test]
fn qa_m0_30_r191_no_orig_check_accepts_a_clean_workspace() {
    check_no_orig(&kernel_unit_use("qa30_orig"));
}

negative_control!(
    qa_m0_30_r191_no_orig_check_accepts_a_clean_workspace,
    "a Cargo.toml.orig left beside kernel's manifest must fail the check",
    expected = "Cargo.toml.orig",
    {
        let root = kernel_unit_use("qa30_orig_nc");
        std::fs::write(root.join("crates/kernel/Cargo.toml.orig"), "").unwrap();
        check_no_orig(&root);
    }
);

/// `check_a_new_target_dir` accepts a directory the run created.
#[test]
fn qa_m0_30_r191_new_target_dir_check_accepts_a_new_directory() {
    let t = scratch("new_dir");
    let before = dirs(&t);
    std::fs::create_dir(t.join("compile-check")).unwrap();
    let after = dirs(&t);
    check_a_new_target_dir(&t, &before, &after);
}

negative_control!(
    qa_m0_30_r191_new_target_dir_check_accepts_a_new_directory,
    "no directory created by the run must fail the check",
    expected = "the compile check left no target directory under",
    {
        let t = scratch("new_dir_nc");
        std::fs::create_dir(t.join("compile-check")).unwrap();
        let before = dirs(&t);
        check_a_new_target_dir(&t, &before, &dirs(&t));
    }
);

/// `check_the_same_target_dir` accepts a target directory holding exactly the first run's directories.
#[test]
fn qa_m0_30_r191_same_target_dir_check_accepts_the_same_directories() {
    let t = scratch("same_dir");
    std::fs::create_dir(t.join("compile-check")).unwrap();
    let after_first = dirs(&t);
    check_the_same_target_dir(&t, &after_first);
}

negative_control!(
    qa_m0_30_r191_same_target_dir_check_accepts_the_same_directories,
    "a second run that adds a directory must fail the stability check",
    expected = "the second run used a different target directory: not stable",
    {
        let t = scratch("same_dir_nc");
        std::fs::create_dir(t.join("compile-check")).unwrap();
        let after_first = dirs(&t);
        std::fs::create_dir(t.join("another")).unwrap();
        check_the_same_target_dir(&t, &after_first);
    }
);
