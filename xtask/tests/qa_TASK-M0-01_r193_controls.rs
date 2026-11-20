//! Negative controls for qa's tests in `qa_TASK-M0-01_r193.rs` (REQ-VAL-007; R-199, R-209, R-212): each runs its
//! test's case, `fails_and_its_control_passes` from the shared module its test calls too (REQ-VAL-161; R-215, R-218),
//! with the unit test's gate moved outside what the compile check compiles: `b` on and `c` off, two features that are
//! not default, which `cargo test --features b` builds and none of the check's six runs does (R-194's known limit).
//! The premise holds, `xtask deps` passes, and the case's check must fail, so the test can fail (philosophy §4.4).
//! Workspaces are named `m022_*`, apart from qa's cases, which write to the same directory.
#![cfg(feature = "controls")]

#[path = "support/qa_m0_01_r193.rs"]
mod qa_m0_01_r193;

use qa_m0_01_r193::*;
use validation::negative_control;

/// Two features, neither default.
const B_AND_C: &str = "\n[features]\nb = []\nc = []\n";

/// `b` on and `c` off: a combination none of the compile check's runs builds.
const OUTSIDE: &str = "feature = \"b\", not(feature = \"c\")";

negative_control!(
    qa_a_unit_test_gated_on_the_host_platform_fails,
    "a unit test of kernel gated on the host platform and on `b` without `c`",
    expected = "builds with validation passes xtask deps (R-187; R-193",
    fails_and_its_control_passes(
        "m022_host",
        "kernel",
        "",
        B_AND_C,
        &format!(
            "all(test, target_os = \"{}\", {OUTSIDE})",
            std::env::consts::OS
        ),
        &["--features", "b"],
    )
);

negative_control!(
    qa_a_unit_test_gated_on_a_missing_feature_fails,
    "a unit test of ledger gated on the missing `c` and on `b`",
    expected = "builds with validation passes xtask deps (R-187; R-193",
    fails_and_its_control_passes(
        "m022_missing_feature",
        "ledger",
        "",
        B_AND_C,
        &format!("all(test, {OUTSIDE})"),
        &["--features", "b"],
    )
);

negative_control!(
    qa_a_unit_test_gated_on_the_release_profile_fails,
    "a unit test of kernel gated on the release profile and on `b` without `c`",
    expected = "builds with validation passes xtask deps (R-187; R-193",
    fails_and_its_control_passes(
        "m022_release",
        "kernel",
        "",
        B_AND_C,
        &format!("all(test, not(debug_assertions), {OUTSIDE})"),
        &["--release", "--features", "b"],
    )
);

negative_control!(
    qa_a_lib_with_test_false_in_every_toml_form_fails,
    "a unit test of ledger, in a lib with `test = false`, gated on `b` without `c`",
    expected = "builds with validation passes xtask deps (R-187; R-193",
    fails_and_its_control_passes(
        "m022_lib_test_false",
        "ledger",
        "",
        &format!("\n[lib]\ntest = false\ndoctest = false\n{B_AND_C}"),
        &format!("all(test, {OUTSIDE})"),
        &["--features", "b"],
    )
);
