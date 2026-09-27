# TASK-M0-25 — Controls for qa's validation and prin tests; subprocess bodies leave libtest

- **Milestone:** M0
- **Closes:** REQ-VAL-153
- **Depends on:** TASK-M0-04, TASK-M0-21
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** PIT-3
- **Size:** ~400–450 lines

## Goal
The second of R-209's three parts of TASK-M0-22. Every qa test merged in `crates/validation` and `crates/prin` before TASK-M0-22 gets a registered negative control that makes it fail (philosophy §4.4, R-199). The three qa child-mode helpers, `#[test]`s that return early unless an env var is set and so cannot fail, move out of libtest (R-210).

## References
- `docs/read_first/principia_00_philosophy.md` § "4.4 A test that cannot fail is not a test"
- `decisions.md` § "R-176 — Controls come before the tests that need them *(closes G3, S2)*"
- `decisions.md` § "R-199 — A test is matched to its control by name in the macro call *(amends R-176; closes RQ-136)*"
- `decisions.md` § "R-209 — TASK-M0-22 is split three ways, each part closing its own requirement *(closes RQ-141)*"
- `decisions.md` § "R-210 — Subprocess bodies leave libtest *(closes RQ-142)*"

## Deliverables
- `crates/prin/Cargo.toml` — the `controls` feature and a dev-dependency on `crates/validation` (R-176).
- A negative control, registered with `negative_control!` (R-199), for each qa test in `crates/validation/tests/` (`qa_TASK-M0-04.rs`, `_r2`, `qa_TASK-M0-21.rs`, `_r2`, `qa_R-206.rs`) and `crates/prin/tests/qa_TASK-M0-01.rs` (25 at R-209, not counting the three child bodies). Each makes its test fail. The controls live in new `*_controls.rs` targets where they can; an edit to qa's merged files is limited to what registration forces, and is reviewed by qa (R-209).
- The child bodies `qa_child_open_harness`, `qa_child_failing_property` (`qa_TASK-M0-04.rs`) and `qa_child_count_cases` (`qa_TASK-M0-04_r2.rs`) moved into a `harness = false` test target (or a bin) whose `main` the parent tests spawn, so they are no longer `#[test]`s. This edits qa's merged files, under R-210's one-round exception; the parent tests keep their assertions (R-210).

## Acceptance tests
- `cargo xtask controls` — on the PR head, each qa test in validation and prin is paired with a control that makes it fail, and none of the three child bodies is listed; the command may still fail naming qa's xtask tests, which TASK-M0-22 covers (REQ-VAL-153).
- `PRIN_GPU_BACKEND=metal cargo test -p validation` — the parent tests that spawn the child bodies pass, and each fails under its control (REQ-VAL-153).
- Review checklist (qa §3): each control is discriminating, and the moved child bodies keep what the parent tests read (REQ-VAL-153).

## Notes
- R-209 (closes RQ-141): TASK-M0-22 is split into TASK-M0-24, this task and TASK-M0-22 itself, which comes last.
- R-210 (closes RQ-142): a subprocess body a test spawns is not itself a `#[test]`.
