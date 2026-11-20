//! QA tests for TASK-M0-21, written from REQ-VAL-147's statement and verify detail (R-176, R-199, R-201) and
//! philosophy §4.4. Each runs `cargo xtask controls` on a copy of a fixture crate in
//! `fixtures/controls_qa_m0_21/`, and carries an inline negative control (R-176; the registry's own controls for
//! these tests are TASK-M0-22's, R-198): the same fixture with one file removed, which must flip the verdict.
//!
//! - `dup_name`: two test targets each hold a test `doubles` and a control named `doubles`; one control leaves its
//!   test passing, so the command must fail naming `doubles` ("fail naming the test when a control leaves its test
//!   passing").
//! - `misnamed`: a control registered under a name that is not the test's does not pair with it (R-199).
//! - `bin_target`: a unit test in the crate's binary target pairs with its control in `tests/` ("across all of a
//!   crate's test targets", R-201).
//! - `workspace`: three crates; pairing stays within a crate, the crate without the feature is skipped and reported.

#[path = "support/qa_m0_21.rs"]
mod qa_m0_21;
#[path = "support/qa_m0_21_fixture.rs"]
mod qa_m0_21_fixture;

use qa_m0_21::*;
use qa_m0_21_fixture::*;

fn has(text: &str, needle: &str) {
    assert!(text.contains(needle), "{needle:?} not in:\n{text}");
}

/// REQ-VAL-147: "fail naming the test when a control leaves its test passing". Two test targets each hold a test
/// `doubles` with a control named `doubles`; the control in `a_leaky.rs` leaves its test passing. The command must
/// fail naming `doubles`: a sound control of the same name in another target must not hide the leaky one.
#[test]
fn qa_leaky_control_beside_a_sound_one_of_the_same_name_fails_naming_the_test() {
    let v = controls("dup_name", &[]);
    check_leaky_fails(&v);
    has(
        &v.stderr,
        "test `doubles`: its control did not make it fail",
    );
    // Control: with the leaky target removed, only the sound control remains and the command passes.
    let sound = controls("dup_name", &["tests/a_leaky.rs"]);
    assert!(
        sound.ok,
        "control: the sound control alone failed the command:\n{}",
        sound.all()
    );
    // Control: with the sound target removed, the leaky control alone is reported, so the fixture does leak.
    let leaky = controls("dup_name", &["tests/b_sound.rs"]);
    assert!(
        !leaky.ok,
        "control: the leaky control alone passed:\n{}",
        leaky.all()
    );
    has(
        &leaky.stderr,
        "test `doubles`: its control did not make it fail",
    );
}

/// R-199: pairing is by the name given in `negative_control!`. A discriminating control registered as `double`
/// does not pair with the test `doubles`, which the command reports as having no control.
#[test]
fn qa_control_under_another_name_does_not_pair() {
    let v = controls("misnamed", &["tests/named.rs"]);
    check_misnamed_fails(&v);
    has(&v.stderr, "test `doubles` has no control");
    // Control: the same control registered as `doubles` pairs, and the command passes.
    let named = controls("misnamed", &["tests/misnamed.rs"]);
    assert!(
        named.ok,
        "control: the control named `doubles` did not pair:\n{}",
        named.all()
    );
    lacks(&named.stderr, "`doubles`");
}

/// R-201, REQ-VAL-147 "across all of a crate's test targets": a unit test in the crate's binary target pairs by
/// name with the control registered in its `tests/`.
#[test]
fn qa_unit_test_in_a_binary_target_pairs_with_its_control_in_tests() {
    let v = controls("bin_target", &[]);
    check_bin_target_pairs(&v);
    has(
        &v.stdout,
        "qa_controls_bin_target: 1 test(s), each failed by its control",
    );
    // Control: without the control in tests/, the binary's unit test is reported by name.
    let bare = controls("bin_target", &["tests/controls.rs"]);
    assert!(
        !bare.ok,
        "control: the binary's unit test passed with no control:\n{}",
        bare.all()
    );
    has(&bare.stderr, "test `bin_tests::quadruples` has no control");
}

/// REQ-VAL-147: pairing is within a crate's own targets, and a crate without the feature is skipped and reported,
/// not failed. In a three-crate workspace, `bare`'s `doubles` has no control even though `controlled` registers
/// one named `doubles`; `plain` is skipped.
#[test]
fn qa_workspace_pairs_within_each_crate_and_skips_the_featureless_one() {
    let v = controls("workspace", &[]);
    check_workspace_fails(&v);
    has(
        &v.stderr,
        "qa_controls_ws_bare: test `doubles` has no control",
    );
    lacks(&v.stderr, "qa_controls_ws_controlled: test");
    lacks(&v.stderr, "qa_controls_ws_plain");
    has(
        &v.stdout,
        "qa_controls_ws_plain: skipped: it declares no `controls` feature",
    );
    has(
        &v.stdout,
        "qa_controls_ws_controlled: 1 test(s), each failed by its control",
    );
    // Control: with `bare`'s test removed, nothing is reported, the skipped crate does not fail the command, and it
    // is still reported skipped.
    let clean = controls("workspace", &["bare/tests/doubles.rs"]);
    assert!(
        clean.ok,
        "control: the workspace failed with every test controlled:\n{}",
        clean.all()
    );
    has(&clean.stdout, "qa_controls_ws_plain: skipped");
}

/// R-176, REQ-VAL-147 "run every control under the `controls` feature": a control compiles only under the
/// calling crate's feature, so an ordinary `cargo test` neither lists nor runs it, and a leaky control does not
/// fail the ordinary suite.
#[test]
fn qa_controls_exist_only_under_the_feature() {
    let copy = Copy::new("dup_name", &[]);
    let test = |features: &[&str]| cargo_test(&copy, features);
    let plain = test(&[]);
    assert!(
        plain.ok,
        "cargo test without the feature failed:\n{}",
        plain.all()
    );
    lacks(&plain.stdout, "negative_control");
    has(&plain.stdout, "test doubles ... ok");
    // Control: under the feature the controls are compiled and run, and the leaky one fails.
    let with = test(&["--features", "controls"]);
    assert!(
        !with.ok,
        "control: the leaky control did not run under the feature:\n{}",
        with.all()
    );
    has(&with.stdout, "doubles::negative_control");
}
