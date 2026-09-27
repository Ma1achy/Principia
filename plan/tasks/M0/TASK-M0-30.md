# TASK-M0-30 — qa's xtask helpers and checks move into shared modules: `_r191`, `_r193`, `_r194`

- **Milestone:** M0
- **Closes:** REQ-VAL-161
- **Depends on:** TASK-M0-04, TASK-M0-21, TASK-M0-24, TASK-M0-25, TASK-M0-26, TASK-M0-27
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** PIT-3
- **Size:** ~420 counted lines per R-211 (part (b2) of RQ-149's split, R-221); moves only

## Goal
The third of R-221's four parts of TASK-M0-22. The helpers and inline checks in qa's `xtask/tests/qa_TASK-M0-01_r191.rs`, `_r193.rs` and `_r194.rs` live in shared test-support modules, so the controls TASK-M0-22 registers in new targets can call the test's own check and share its inputs (R-212, R-215, R-218). Nothing about the tests' behaviour changes.

## References
- `docs/read_first/principia_00_philosophy.md` § "4.4 A test that cannot fail is not a test"
- `decisions.md` § "R-212 — A control names the panic it expects *(amends R-199; closes RQ-145)*"
- `decisions.md` § "R-215 — The veto items on PRs #23 and #24 stand; duplicated controls and copied checks are consolidated"
- `decisions.md` § "R-218 — Inputs a control shares with its test live in the shared module too *(extends R-215)*"
- `decisions.md` § "R-221 — TASK-M0-22 is split four ways under R-208 *(closes RQ-149)*"

## Deliverables
- Shared test-support modules under `xtask/tests/support/`, holding the helpers and checks of `xtask/tests/qa_TASK-M0-01_r191.rs`, `_r193.rs` and `_r194.rs` that the tests and their controls both need, each item used by every file that includes its module (no dead code). Each check a test makes inline becomes a function the test calls. An edit to qa's merged files is limited to replacing a helper or an inline check with a call to the module, under R-215's one-round exception, with qa reviewing.

## Acceptance tests
- `cargo test -p xtask` — passes, and `cargo test -p xtask -- --list` lists the same tests as on main (REQ-VAL-161).
- Review checklist (code and qa): every helper and check the tests' controls will need is callable from a support module; qa's merged files change only where a helper or check is replaced by a call (REQ-VAL-161).

## Notes
- R-221 (closes RQ-149): part (b2) of the split. RQ-149 counted the helpers to move from the files' non-blank lines at `f71765e`.
- The r193 and r194 helpers are near-duplicates. Merging them would edit qa's calls beyond R-215's exception, so they stay in separate modules.
