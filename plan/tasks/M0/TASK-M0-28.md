# TASK-M0-28 — The implementer's duplicated controls and copied checks, and the shared shader text

- **Milestone:** M0
- **Closes:** REQ-VAL-158, REQ-VAL-159
- **Depends on:** TASK-M0-04, TASK-M0-21, TASK-M0-24, TASK-M0-25, TASK-M0-26, TASK-M0-27
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** PIT-3, PIT-9
- **Size:** ~400 counted lines per R-211 (part (a) of RQ-149's split, R-221; ~165 of them deletions)

## Goal
The first of R-221's four parts of TASK-M0-22. The implementer's tests keep no inline control that a registered one duplicates, and no controls target copies a check or an input its test uses: the `gpu` and `prop` unit-test checks, and the GPU shader text qa's M0-04 test and its control both need, each live once (R-215, R-218).

## References
- `docs/read_first/principia_00_philosophy.md` § "4.4 A test that cannot fail is not a test"
- `decisions.md` § "R-216 — TASK-M0-22 is split three ways under R-211 *(closes RQ-148)*"
- `decisions.md` § "R-212 — A control names the panic it expects *(amends R-199; closes RQ-145)*"
- `decisions.md` § "R-215 — The veto items on PRs #23 and #24 stand; duplicated controls and copied checks are consolidated"
- `decisions.md` § "R-218 — Inputs a control shares with its test live in the shared module too *(extends R-215)*"
- `decisions.md` § "R-221 — TASK-M0-22 is split four ways under R-208 *(closes RQ-149)*"

## Deliverables
- In the implementer's tests, the inline controls duplicated by registered ones removed (`xtask/tests/controls.rs`, `deps.rs`, `src/gpu.rs`, `src/prop.rs`), and `crates/validation/tests/controls.rs`'s copies of the `gpu` and `prop` unit-test checks replaced by the tests' own checks (R-215, R-216).
- The GPU shader text duplicated in `crates/validation/tests/qa_TASK-M0-04_controls.rs` moved into the shared
  test-support module, used by the test and its control alike; an edit to qa's merged files only replaces the copy with
  a reference to the shared item, under R-215's one-round exception, with qa reviewing (R-218).

## Acceptance tests
- `PRIN_GPU_BACKEND=metal cargo test -p validation -p xtask --features validation/controls,xtask/controls` — every test and its control pass, each control tripping its expected message (REQ-VAL-158, REQ-VAL-159).
- Review checklist (code and qa): no implementer test keeps an inline control its registered one duplicates; no check body is copied into a controls target; `cargo xtask controls` shows every registered control still trips its test's assertion (REQ-VAL-158).
- Review checklist (code and qa): no input the test and its control both need (shader text, fixture, constant) is
  copied between qa's test files and the controls targets (REQ-VAL-159).

## Notes
- R-221 (closes RQ-149): part (a) of the split, moved here from TASK-M0-22.
- qa's `crates/validation/tests/qa_TASK-M0-24.rs` (`run_controls`, ~line 222) runs `cargo test -p validation --lib --test controls`, so `crates/validation/tests/controls.rs` stays; RQ-149 records the implementer's plan to reach the gpu and prop checks from it through a `#[cfg(any(test, feature = "controls"))]` module.
