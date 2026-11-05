//! QA tests for TASK-M8-86, written from REQ-TOOL-220's verify detail, "its deploy job runs only when `github.ref` is
//! `refs/heads/main`": the condition is the deploy *job's*. A ref condition on one of its steps, inside a script's text,
//! under its environment, or on the build job instead, leaves the deploy job itself running on any ref, so the check of
//! pages.yml must refuse each.
// The file name `qa_TASK-M8-86_job_condition` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::path::Path;

use validation::negative_control;
use xtask::pages_workflow::check;

/// The deploy job's own condition, as the deliverable states it.
const JOB_CONDITION: &str = "    if: github.ref == 'refs/heads/main'\n";

fn workflow() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../.github/workflows/pages.yml");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The real file with the job's condition removed and `from` replaced by `to`.
fn moved(from: &str, to: &str) -> String {
    let text = workflow();
    assert!(
        text.contains(JOB_CONDITION),
        "the deploy job's condition is not in pages.yml"
    );
    assert!(
        text.contains(from),
        "the control's `{from}` is not in pages.yml"
    );
    text.replacen(JOB_CONDITION, "", 1).replacen(from, to, 1)
}

/// Each place the condition could sit that is not the deploy job's own.
fn check_condition_faults(check_one: impl Fn(&str, &str)) {
    check_one(
        &moved(
            "      - id: deploy\n",
            "      - id: deploy\n        if: github.ref == 'refs/heads/main'\n",
        ),
        "the condition on the deploy step, the job running on any ref",
    );
    check_one(
        &moved(
            "    steps:\n      - id: deploy\n",
            "    steps:\n      - run: |\n          echo on\n          if: github.ref == 'refs/heads/main'\n      - id: deploy\n",
        ),
        "the condition's text inside a script of the deploy job",
    );
    check_one(
        &moved(
            "      name: github-pages\n",
            "      name: github-pages\n      if: github.ref == 'refs/heads/main'\n",
        ),
        "the condition under the environment",
    );
    check_one(
        &moved(
            "  build:\n    runs-on: ubuntu-24.04\n",
            "  build:\n    if: github.ref == 'refs/heads/main'\n    runs-on: ubuntu-24.04\n",
        ),
        "the condition on the build job, not the deploy job",
    );
    check_one(
        &moved("    needs: build\n", "    needs: build\n    if: always()\n"),
        "the deploy job conditioned on something else",
    );
}

fn refused(text: &str, what: &str) {
    assert!(check(text).is_err(), "pages.yml passed with {what}");
}

#[test]
fn qa_m8_86_the_ref_condition_is_the_deploy_jobs_own() {
    check_condition_faults(refused);
}

negative_control!(
    qa_m8_86_the_ref_condition_is_the_deploy_jobs_own,
    "the real file offered as each fault",
    expected = "pages.yml passed with",
    refused(&workflow(), "the real file")
);

/// The real file, the condition kept where it is, passes: the faults above are refused for where the condition went, not
/// for some other edit.
#[test]
fn qa_m8_86_the_job_condition_in_place_passes() {
    let text = workflow();
    assert!(text.contains(JOB_CONDITION), "the deploy job's condition");
    assert_eq!(check(&text), Ok(()));
}

negative_control!(
    qa_m8_86_the_job_condition_in_place_passes,
    "the condition removed, put nowhere",
    expected = "not conditioned on refs/heads/main",
    assert_eq!(
        check(&moved("    needs: build\n", "    needs: build\n")),
        Ok(())
    )
);
