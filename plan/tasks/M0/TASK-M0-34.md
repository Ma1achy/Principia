# TASK-M0-34 — A failing control's output kept in qa's message; the deps.rs control checks its compile error first

- **Milestone:** M0
- **Closes:** REQ-VAL-167
- **Depends on:** TASK-M0-22
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** PIT-3
- **Size:** per R-223/R-225, est. ~40 counted lines

## Goal
When a control fails to make its test fail, the reason is kept. During PR #38's review one control in `xtask/tests/deps.rs` left its test passing once in five cold runs, and the cause was lost because qa's `qa_TASK-M0-24.rs` drops the failing control's output (R-236).

## References
- `docs/read_first/principia_00_philosophy.md` § "4.4 A test that cannot fail is not a test"
- `decisions.md` § "R-212 — A control names the panic it expects *(amends R-199; closes RQ-145)*"
- `decisions.md` § "R-236 — A failing control's output is kept, and the `deps.rs:1205` control checks its compile error first"
- `decisions.md` § "R-244 — TASK-M0-34's first deliverable moves to `xtask/src/controls.rs` *(amends R-236; closes RQ-155)*"

## Deliverables
- `xtask/src/controls.rs`: when a control fails to make its test fail (`ControlPasses`, `WrongPanic`), the finding includes that control's own output and no longer says it "leaves it passing". The one-round exception covers qa's `xtask/tests/qa_TASK-M0-22.rs`, `crates/validation/tests/qa_TASK-M0-21.rs` and `xtask/tests/controls.rs`, limited to their assertions on that wording, with qa reviewing (R-236, R-244).
- `xtask/tests/deps.rs`: the control of `deps_a_unit_test_behind_the_release_profile_using_validation_fails` first asserts the expected compile error, with its own message, before its test's check (R-236).

## Acceptance tests
- `cargo xtask controls` — every control still trips its test (REQ-VAL-167).
- Review checklist (code and qa): a scratch control that fails its test with the wrong panic shows that control's output in the message; the deps.rs control fails with its own message when the child does not fail to compile with the expected error (REQ-VAL-167).

## Notes
- R-236: from the diagnosis of 29 Sep: not reproduced in ~340 runs, likely an environmental transient; these changes keep the evidence next time. Depends on TASK-M0-22, which also edits `qa_TASK-M0-24.rs` (applied per R-204, sequencing).
- R-244 (closes RQ-155): TASK-M0-22 had removed the control-running code from `qa_TASK-M0-24.rs`; the message lives in `xtask/src/controls.rs`, so the deliverable moves there.
