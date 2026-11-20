//! Negative controls for qa's tests in `qa_TASK-M0-01_r194.rs` (REQ-VAL-007; R-199, R-209, R-212): each runs its
//! test's checks, from the shared module its test calls too (REQ-VAL-161; R-215, R-218), on a workspace they must
//! reject, so the test can fail (philosophy §4.4). A matrix case keeps its test's features and profile, with the unit
//! test's gate moved to a combination outside the 3×2 matrix (R-194's known limit): its cell's premises hold, `xtask
//! deps` passes, and the case's check must fail. Workspaces are named `m022_*`, apart from qa's cases, which write to
//! the same directory.
#![cfg(feature = "controls")]

#[path = "support/qa_m0_01_r194.rs"]
mod qa_m0_01_r194;

use qa_m0_01_r194::*;
use validation::negative_control;

negative_control!(
    qa_a_unit_test_seen_only_with_default_features_in_dev_fails,
    "a unit test of kernel under `b` without a third feature `c`, in dev, which no run of the matrix builds",
    expected = "builds with validation passes xtask deps (R-187; R-194",
    fails_and_its_control_passes(
        "m022_b_without_c_dev",
        "kernel",
        &format!("{A_DEFAULT_B_NOT}c = []\n"),
        "all(test, feature = \"b\", not(feature = \"c\"))",
        &["--features", "b"],
        &[&[]],
    )
);

negative_control!(
    qa_a_unit_test_seen_only_with_default_features_in_release_fails,
    "a unit test of ledger under `b` without a third feature `c`, in release, which no run of the matrix builds",
    expected = "builds with validation passes xtask deps (R-187; R-194",
    fails_and_its_control_passes(
        "m022_b_without_c_release",
        "ledger",
        &format!("{A_DEFAULT_B_NOT}c = []\n"),
        "all(test, feature = \"b\", not(feature = \"c\"), not(debug_assertions))",
        &["--release", "--features", "b"],
        &[&["--features", "b"]],
    )
);

negative_control!(
    qa_a_unit_test_seen_only_with_all_features_in_release_fails,
    "a unit test of kernel under `x` without a second feature `y`, in release, which no run of the matrix builds",
    expected = "builds with validation passes xtask deps (R-187; R-194",
    fails_and_its_control_passes(
        "m022_x_without_y_release",
        "kernel",
        &format!("{X_NOT_DEFAULT}y = []\n"),
        "all(test, feature = \"x\", not(feature = \"y\"), not(debug_assertions))",
        &["--release", "--features", "x"],
        &[&["--features", "x"]],
    )
);

negative_control!(
    qa_a_ledger_doctest_using_validation_passes,
    "a ledger whose doctest runs and whose unit test uses validation, given to the doctest check",
    expected = "a ledger doctest that uses validation fails xtask deps",
    {
        let lib = format!(
            "/// ```\n/// assert_eq!(1 + 1, 2);\n/// ```\n{}",
            lib_rs("test", true)
        );
        let root = workspace(
            "m022_doctest_and_unit_use",
            &[
                (
                    "crates/ledger/Cargo.toml".to_owned(),
                    crate_manifest("ledger", ""),
                ),
                ("crates/ledger/src/lib.rs".to_owned(), lib),
            ],
        );
        premise_the_doctest_runs(&root);
        check_the_unit_test_use_fails(&root);
        check_the_doctest_passes(&root);
    }
);
