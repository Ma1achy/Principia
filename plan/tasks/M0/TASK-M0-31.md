# TASK-M0-31 — The two flaky qa tests fixed at their cause

- **Milestone:** M0
- **Closes:** REQ-VAL-162
- **Depends on:** TASK-M0-28
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** PIT-3
- **Size:** small (R-224: "one small follow-up")

## Goal
Two merged qa tests each failed once under load and passed on rerun. A test that fails for a reason other than its claim shows nothing (philosophy §4.4), and a retry would hide the cause. Each is fixed at its cause, and the workspace suite then passes 20 times in a row under load (R-224).

## References
- `docs/read_first/principia_00_philosophy.md` § "4.4 A test that cannot fail is not a test"
- `decisions.md` § "R-208 — TASK-M0-21 is accepted at 762 code lines; later overruns are split first"
- `decisions.md` § "R-215 — The veto items on PRs #23 and #24 stand; duplicated controls and copied checks are consolidated"
- `decisions.md` § "R-224 — The two flaky tests are fixed before TASK-M0-29"

## Deliverables
- `qa_m0_26_a_control_without_an_expected_message_does_not_compile` (`crates/validation/tests/qa_TASK-M0-26.rs`) and its control build their fixture in separate target directories, as R-208 did for `deps.rs`, so neither reuses the other's build.
- `qa_m0_25_moved_failing_property_reports_a_draw_the_property_fails_on` (`crates/validation/tests/qa_TASK-M0-25.rs`): the cause of its failure under load found (timing, a shared resource, a timeout), shown in the PR with the evidence, and fixed at that cause. No retry.
- The edits to qa's merged files are limited to what the fix needs, under a one-round exception, with qa reviewing (R-224, as R-215).

## Acceptance tests
- The whole workspace suite (`PRIN_GPU_BACKEND=metal cargo test --workspace --features validation/controls,prin/controls,xtask/controls`) passes 20 times in a row under load, with no failures; the PR shows the loop, the load used and each run's result (REQ-VAL-162).
- Review checklist (code and qa): the qa_m0_26 test and its control use separate target directories; the PR names the qa_m0_25 cause with evidence and the fix removes it; no retry is added (REQ-VAL-162).

## Notes
- R-224: the human ordered this before TASK-M0-29, so TASK-M0-29 depends on it.
- Reported on PR #31: the qa_m0_26 fixture build finished in 0.28 s, a reused build, while the test and its control built the same package into one `CARGO_TARGET_DIR`; the qa_m0_25 test failed once in a full-workspace run and passed on two reruns.
