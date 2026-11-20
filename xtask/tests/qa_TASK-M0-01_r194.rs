//! QA tests for TASK-M0-01 after R-194, written from REQ-SYS-004's statement and verify detail — "runs cargo check
//! -p kernel -p ledger --lib --tests six times, with --no-default-features, with default features and with
//! --all-features, each in the dev and the release profile (R-194, amending R-192, so a unit test behind a feature,
//! behind a feature's absence or behind the release profile is compiled), and fails, showing the compiler's error ...;
//! any cfg combination outside that 3×2 matrix ... is not seen, a known limit too (R-194); a kernel or ledger doctest
//! may use validation, and the check does not compile doctests (R-194)" — not from the implementation.
//!
//! Each of the six runs must count. A unit test that uses validation is placed where exactly one cell of the 3×2
//! matrix compiles it, for the cells the implementer's own tests do not isolate:
//! - default features, dev: `all(feature = "a", not(feature = "b"))`, `default = ["a"]` (no-default has `a` off,
//!   all-features has `b` on);
//! - default features, release: the same gate and `not(debug_assertions)`;
//! - all features, release: `all(feature = "x", not(debug_assertions))`, `x` not default.
//!
//! Each failing case first shows its premise: `cargo test` with the flags of that cell builds and runs the unit test
//! with the dev-dependency, so it is a unit test that uses validation, inside the matrix. Control for each (R-176):
//! the same workspace with the use removed passes `xtask deps` with the compile check run.
//!
//! Doctests: a ledger doctest that uses validation passes the check (premise: `cargo test --doc` runs it with the
//! dev-dependency); negative control: the same use in a ledger unit test fails.
#![allow(non_snake_case)]

#[path = "support/qa_m0_01_r194.rs"]
mod qa_m0_01_r194;

use qa_m0_01_r194::*;

/// Default features, dev profile: only that cell compiles a unit test under `a` on and `b` off, `a` default.
#[test]
fn qa_a_unit_test_seen_only_with_default_features_in_dev_fails() {
    let cfg = "all(test, feature = \"a\", not(feature = \"b\"))";
    let others: &[&[&str]] = &[&["--features", "b"]];
    fails_and_its_control_passes(
        "default_dev_kernel",
        "kernel",
        A_DEFAULT_B_NOT,
        cfg,
        &[],
        others,
    );
    fails_and_its_control_passes(
        "default_dev_ledger",
        "ledger",
        A_DEFAULT_B_NOT,
        cfg,
        &[],
        others,
    );
}

/// Default features, release profile: only that cell compiles a unit test under `a` on, `b` off and
/// `not(debug_assertions)`.
#[test]
fn qa_a_unit_test_seen_only_with_default_features_in_release_fails() {
    let cfg = "all(test, feature = \"a\", not(feature = \"b\"), not(debug_assertions))";
    let others: &[&[&str]] = &[&[], &["--release", "--features", "b"]];
    fails_and_its_control_passes(
        "default_release_kernel",
        "kernel",
        A_DEFAULT_B_NOT,
        cfg,
        &["--release"],
        others,
    );
}

/// All features, release profile: only that cell compiles a unit test under non-default `x` and
/// `not(debug_assertions)`.
#[test]
fn qa_a_unit_test_seen_only_with_all_features_in_release_fails() {
    let cfg = "all(test, feature = \"x\", not(debug_assertions))";
    let others: &[&[&str]] = &[&["--release"], &["--features", "x"]];
    fails_and_its_control_passes(
        "all_release_kernel",
        "kernel",
        X_NOT_DEFAULT,
        cfg,
        &["--release", "--all-features"],
        others,
    );
    fails_and_its_control_passes(
        "all_release_ledger",
        "ledger",
        X_NOT_DEFAULT,
        cfg,
        &["--release", "--all-features"],
        others,
    );
}

/// R-194: a ledger doctest may use validation. Premise: `cargo test --doc` runs the doctest with the dev-dependency.
/// Negative control: the same use in a ledger unit test fails the check.
#[test]
fn qa_a_ledger_doctest_using_validation_passes() {
    let toml = "crates/ledger/Cargo.toml".to_owned();
    let lib = "crates/ledger/src/lib.rs".to_owned();
    let cargo = crate_manifest("ledger", "");
    let doc = "/// ```\n/// let _ = validation::Harness;\n/// ```\npub fn f() {}\n".to_owned();
    let root = workspace(
        "doctest_ledger",
        &[(toml.clone(), cargo.clone()), (lib.clone(), doc)],
    );

    premise_the_doctest_runs(&root);
    check_the_doctest_passes(&root);

    let root = workspace(
        "doctest_ledger_control",
        &[(toml, cargo), (lib, lib_rs("test", true))],
    );
    check_the_unit_test_use_fails(&root);
}
