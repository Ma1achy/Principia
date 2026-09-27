# TASK-M0-27 — qa's copied checks move into shared test-support modules

- **Milestone:** M0
- **Closes:** REQ-VAL-157
- **Depends on:** TASK-M0-26
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** PIT-3
- **Size:** ~540 counted lines per R-211, about half of them deletions of the copies (part 4 of RQ-148's split; the human chose the three-task option that showed it at ~540, R-216)

## Goal
The second of R-216's three parts of TASK-M0-22. The checks and helpers TASK-M0-25's controls copied from qa's test files live once, in shared test-support modules that each test and its control both call (R-215), so a control can't drift from the check it controls.

## References
- `docs/read_first/principia_00_philosophy.md` § "4.4 A test that cannot fail is not a test"
- `decisions.md` § "R-212 — A control names the panic it expects *(amends R-199; closes RQ-145)*"
- `decisions.md` § "R-215 — The veto items on PRs #23 and #24 stand; duplicated controls and copied checks are consolidated"
- `decisions.md` § "R-216 — TASK-M0-22 is split three ways under R-211 *(closes RQ-148)*"

## Deliverables
- Shared test-support modules for the checks and helpers copied between `crates/validation/tests/qa_TASK-M0-04.rs`/`_r2` and `qa_TASK-M0-04_controls.rs` (diff_at, child, marker and the checks), `qa_TASK-M0-21.rs`/`_r2` and `qa_TASK-M0-21_controls.rs` (fixture copy, xtask run and the checks), and prin's `qa_TASK-M0-01.rs` and `qa_TASK-M0-01_controls.rs`; the copies removed. An edit to qa's merged files is limited to replacing a copied check with a call to the module, under R-215's one-round exception, with qa reviewing.

## Acceptance tests
- `PRIN_GPU_BACKEND=metal cargo test -p validation -p prin --features validation/controls,prin/controls` — every qa test and its control pass, each control tripping its expected message (REQ-VAL-157).
- Review checklist (code and qa): no check body is copied between qa's test files and the controls targets; qa's merged files change only where a copy is replaced by a call (REQ-VAL-157).

## Notes
- R-216 (closes RQ-148): part 4 of the split, ~540 counted lines, about half of them deletions; the human chose it as its own task.
