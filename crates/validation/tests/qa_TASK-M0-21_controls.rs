//! Negative controls for qa's tests in `qa_TASK-M0-21.rs` and `qa_TASK-M0-21_r2.rs` (REQ-VAL-153; R-199, R-209).
//! Each runs its test's check, called from the shared modules its test calls too (REQ-VAL-157; R-215), on a fixture in
//! `fixtures/controls_qa_m0_21/` with one file removed, which flips the verdict the check must reject, so the test can
//! fail (philosophy §4.4).
#![cfg(feature = "controls")]

#[path = "support/qa_m0_21.rs"]
mod qa_m0_21;
#[path = "support/qa_m0_21_fixture.rs"]
mod qa_m0_21_fixture;
#[path = "support/qa_m0_21_r2.rs"]
mod qa_m0_21_r2;

use qa_m0_21::*;
use qa_m0_21_fixture::*;
use qa_m0_21_r2::*;
use validation::negative_control;

negative_control!(
    qa_leaky_control_beside_a_sound_one_of_the_same_name_fails_naming_the_test,
    "with the leaky control removed, the command passes, so the must-fail check must fail",
    expected = "a control that leaves `doubles` passing passed the command",
    {
        check_leaky_fails(&controls("dup_name", &["tests/a_leaky.rs"]));
    }
);

negative_control!(
    qa_control_under_another_name_does_not_pair,
    "with the misnamed control removed, the rightly named one pairs, so the must-fail check must fail",
    expected = "a test whose only control names another test passed",
    {
        check_misnamed_fails(&controls("misnamed", &["tests/misnamed.rs"]));
    }
);

negative_control!(
    qa_unit_test_in_a_binary_target_pairs_with_its_control_in_tests,
    "with the control in tests/ removed, the must-pass check must fail",
    expected = "the binary's unit test did not pair with its control",
    {
        check_bin_target_pairs(&controls("bin_target", &["tests/controls.rs"]));
    }
);

negative_control!(
    qa_workspace_pairs_within_each_crate_and_skips_the_featureless_one,
    "with bare's uncontrolled test removed, the command passes, so the must-fail check must fail",
    expected = "a test paired with a control in another crate",
    {
        check_workspace_fails(&controls("workspace", &["bare/tests/doubles.rs"]));
    }
);

negative_control!(
    qa_binary_only_crate_is_checked_and_passes,
    "with the control in tests/ removed, the must-pass check must fail",
    expected = "a binary-only crate whose one test has a discriminating control failed the command",
    {
        check_bin_only_passes(&controls("bin_only", &["tests/controls.rs"]));
    }
);

negative_control!(
    qa_controls_exist_only_under_the_feature,
    "under the feature the sound control compiles and runs, so the no-control check must fail",
    expected = "\"negative_control\" in:",
    {
        let copy = Copy::new("dup_name", &["tests/a_leaky.rs"]);
        let with = cargo_test(&copy, &["--features", "controls"]);
        assert!(
            with.ok,
            "cargo test with the feature failed:\n{}",
            with.all()
        );
        lacks(&with.stdout, "negative_control");
    }
);
