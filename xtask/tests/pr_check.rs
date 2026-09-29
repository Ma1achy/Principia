//! `cargo xtask pr-check` (REQ-SYS-006, REQ-VAL-001, REQ-VAL-005, REQ-VAL-008; R-180): per label, a complete body
//! passes and an incomplete one fails naming the section, question or item.

use std::path::{Path, PathBuf};
use std::process::Command;

use validation::negative_control;
use xtask::pr_check::{check, PullRequest};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/fixtures/{name}.json"))
}

/// The problems pr-check finds in the event fixture `name`.
fn problems(name: &str) -> Vec<String> {
    let pr = PullRequest::from_event(&fixture(name)).expect("fixture read");
    check(&pr.body, &pr.labels)
}

fn passes(name: &str) {
    let found = problems(name);
    assert!(found.is_empty(), "pr-check failed {name}: {found:?}");
}

/// Exactly one problem, and it contains each of `names`.
fn fails_naming(name: &str, names: &[&str]) {
    let found = problems(name);
    assert!(
        found.len() == 1 && names.iter().all(|n| found[0].contains(n)),
        "pr-check did not fail naming {names:?}: {found:?}"
    );
}

#[test]
fn pr_check_design_complete_body_passes() {
    passes("pr_design_complete");
}

#[test]
fn pr_check_design_missing_answer_fails_naming_the_question() {
    fails_naming(
        "pr_design_incomplete",
        &["`Could this measurement have failed?`"],
    );
}

#[test]
fn pr_check_validation_meter_and_discriminator_complete_body_passes() {
    passes("pr_validation_complete");
}

#[test]
fn pr_check_validation_meter_without_integrated_quantity_fails_naming_it() {
    fails_naming(
        "pr_validation_meter_incomplete",
        &["meter `COM drift`", "the quantity the occupant integrates"],
    );
}

#[test]
fn pr_check_validation_discriminator_without_termination_dependency_fails_naming_it() {
    fails_naming(
        "pr_validation_discriminator_incomplete",
        &["discriminator `d_min`", "its dependency on termination"],
    );
}

#[test]
fn pr_check_investigation_entry_or_reason_passes() {
    passes("pr_investigation_complete");
    passes("pr_investigation_reason");
}

#[test]
fn pr_check_investigation_without_entry_or_reason_fails() {
    fails_naming("pr_investigation_incomplete", &["investigation: neither"]);
}

/// The template unfilled, under every label: each labelled section is empty, its guidance being comments.
#[test]
fn pr_check_unfilled_template_fails_naming_every_labelled_section() {
    let found = problems("pr_all_labels_template");
    let expected = [
        "section `Validation record` is empty (label `validation`)",
        "section `Investigation` is empty (label `investigation`)",
        "design question `Which path is this for?` is unanswered",
        "design question `Does the instrument still say where it stops knowing?` is unanswered",
    ];
    assert!(
        found.len() == 6 && expected.iter().all(|e| found.iter().any(|f| f.contains(e))),
        "unfilled template not failed on every labelled section: {found:?}"
    );
}

/// Without its label a section is not mandatory; with it, a body lacking the section fails naming it.
#[test]
fn pr_check_labels_select_mandatory_sections() {
    assert!(check("", &[]).is_empty(), "an unlabelled empty body failed");
    let found = check(
        "## Task and Closes\nTASK-M0-99\n",
        &["validation".to_owned()],
    );
    assert_eq!(
        found,
        ["section `Validation record` is missing (label `validation`)"],
        "missing section not named"
    );
}

/// `cargo xtask pr-check --event <file>`: exit status 0 on a complete body, and non-zero naming the problem otherwise.
fn run_binary(name: &str) -> (bool, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["pr-check", "--event"])
        .arg(fixture(name))
        .output()
        .expect("xtask ran");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn pr_check_binary_reads_the_event() {
    let (ok, stderr) = run_binary("pr_design_complete");
    assert!(ok, "complete body failed: {stderr}");
    let (ok, stderr) = run_binary("pr_design_incomplete");
    assert!(
        !ok && stderr.contains("`Could this measurement have failed?`"),
        "incomplete body not failed naming the question: {stderr}"
    );
}

negative_control!(
    pr_check_design_complete_body_passes,
    "a design body with an unanswered drift question must fail the passing check",
    expected = "pr-check failed",
    passes("pr_design_incomplete")
);

negative_control!(
    pr_check_design_missing_answer_fails_naming_the_question,
    "a design body answering every question gives no problem to name",
    expected = "pr-check did not fail naming",
    fails_naming(
        "pr_design_complete",
        &["`Could this measurement have failed?`"]
    )
);

negative_control!(
    pr_check_validation_meter_and_discriminator_complete_body_passes,
    "a validation body with a meter stating nothing must fail the passing check",
    expected = "pr-check failed",
    passes("pr_validation_meter_incomplete")
);

negative_control!(
    pr_check_validation_meter_without_integrated_quantity_fails_naming_it,
    "a meter stating its integrated quantity gives no problem to name",
    expected = "pr-check did not fail naming",
    fails_naming(
        "pr_validation_complete",
        &["meter `COM drift`", "the quantity the occupant integrates"]
    )
);

negative_control!(
    pr_check_validation_discriminator_without_termination_dependency_fails_naming_it,
    "a discriminator stating its termination dependency gives no problem to name",
    expected = "pr-check did not fail naming",
    fails_naming(
        "pr_validation_complete",
        &["discriminator `d_min`", "its dependency on termination"]
    )
);

negative_control!(
    pr_check_investigation_entry_or_reason_passes,
    "an investigation body with neither entry nor reason must fail the passing check",
    expected = "pr-check failed",
    passes("pr_investigation_incomplete")
);

negative_control!(
    pr_check_investigation_without_entry_or_reason_fails,
    "an investigation body stating its reason gives no problem to name",
    expected = "pr-check did not fail naming",
    fails_naming("pr_investigation_reason", &["investigation: neither"])
);

negative_control!(
    pr_check_unfilled_template_fails_naming_every_labelled_section,
    "a complete design body under one label gives none of the six problems",
    expected = "unfilled template not failed on every labelled section",
    {
        let found = problems("pr_design_complete");
        assert!(
            found.len() == 6,
            "unfilled template not failed on every labelled section: {found:?}"
        );
    }
);

negative_control!(
    pr_check_labels_select_mandatory_sections,
    "a body with the section present gives no missing-section problem",
    expected = "missing section not named",
    {
        let found = check(
            "## Validation record\n- meter: E — the energy the integrator advances\n",
            &["validation".to_owned()],
        );
        assert_eq!(
            found,
            ["section `Validation record` is missing (label `validation`)"],
            "missing section not named"
        );
    }
);

negative_control!(
    pr_check_binary_reads_the_event,
    "the binary run on an incomplete body must fail the passing check",
    expected = "complete body failed",
    {
        let (ok, stderr) = run_binary("pr_validation_meter_incomplete");
        assert!(ok, "complete body failed: {stderr}");
    }
);
