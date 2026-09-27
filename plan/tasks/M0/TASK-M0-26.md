# TASK-M0-26 — The child-spawn helper with a timeout, and controls that name their expected panic

- **Milestone:** M0
- **Closes:** REQ-VAL-154, REQ-VAL-155, REQ-VAL-156
- **Depends on:** TASK-M0-24, TASK-M0-25
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** PIT-3
- **Size:** ~350 counted lines per R-211 (parts 1 and 2 of RQ-148's split, R-216)

## Goal
The first of R-216's three parts of TASK-M0-22. Every child process a test spawns goes through one helper that fails naming the child when it outlives a timeout, instead of stalling the suite (R-214), and `qa_r206_harness_opens_the_selected_backend`'s child path leaves libtest (R-213). `negative_control!` takes the panic message its control expects, so a control passes only by tripping its intended assertion (R-212), and every existing control is converted.

## References
- `docs/read_first/principia_00_philosophy.md` § "4.4 A test that cannot fail is not a test"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-182 — Escape fixtures are defined; proposed tolerances are provisional in CI *(closes T4, T5)*"
- `decisions.md` § "R-199 — A test is matched to its control by name in the macro call *(amends R-176; closes RQ-136)*"
- `decisions.md` § "R-210 — Subprocess bodies leave libtest *(closes RQ-142)*"
- `decisions.md` § "R-212 — A control names the panic it expects *(amends R-199; closes RQ-145)*"
- `decisions.md` § "R-213 — R-210's exception extends to `qa_r206_harness_opens_the_selected_backend` *(closes RQ-146)*"
- `decisions.md` § "R-214 — Children are spawned through one helper with a timeout *(closes RQ-147)*"
- `decisions.md` § "R-216 — TASK-M0-22 is split three ways under R-211 *(closes RQ-148)*"
- `decisions.md` § "R-217 — TASK-M0-26's size accepted; a timed-out child's whole process group dies; the timeout is 300 s provisional *(amends R-214)*"

## Deliverables
- A shared child-spawn helper in `crates/validation`: it waits at most the timeout (300 s, provisional, REQ-VAL-156, R-217) and on timeout kills the child and fails naming it; on Unix the child runs in its own process group, and on timeout the whole group gets SIGTERM, then SIGKILL after a 5 s grace, and is reaped (R-217). Every test that spawns a child uses it, qa's merged files included, under R-214's one-round exception limited to replacing the spawn call (R-214).
- `qa_r206_harness_opens_the_selected_backend`'s child path moved into the `qa_child` bin, so it is no longer reached through a `#[test]` (R-213, R-210's exception).
- `negative_control!` takes the panic message its control expects and registers it with `#[should_panic(expected = …)]` (R-212; the form in the call is this task's to choose, stated in the PR). Every control registered before this task — TASK-M0-21's fixtures, TASK-M0-24's, TASK-M0-25's, qa's and PR #20's — is converted to it, with messages added to the assertions a control must name. `cargo xtask controls` reports a control that panics with the wrong message.

## Acceptance tests
- `cargo test -p validation spawn` — a child that outlives a short test timeout is killed, the helper's error names it and the killed child no longer exists; a child that exits in time returns its output (REQ-VAL-155).
- `cargo test -p validation -- --list` — lists no child path of `qa_R-206.rs`: `qa_r206_harness_opens_the_selected_backend` is a test only, its child body in the `qa_child` bin (R-213) (REQ-VAL-155).
- `cargo test -p validation --features controls` — a fixture control that panics in its setup fails; with its expected message reaching the check it passes (REQ-VAL-154).
- `cargo xtask controls` — every control in the workspace (validation, prin, xtask) carries an expected message, which the macro requires, and trips it; the command may still fail naming qa's xtask tests, which TASK-M0-22 covers (REQ-VAL-154).
- `cargo test -p validation spawn` — a child that starts a grandchild and then hangs: after the timeout both are gone and no process from its group remains (R-217) (REQ-VAL-155).
- Proposal (REQ-VAL-156): the helper's 300 s timeout (R-217) with its evidence (the longest child run measured in the suite, cold and warm, and the headroom), provisional until the human confirms it at the M0 gate.
- Review checklist (code and qa): no test spawns a child except through the helper (REQ-VAL-155).

## Notes
- R-216 (closes RQ-148): TASK-M0-22 is split into this task, TASK-M0-27 and TASK-M0-22 itself, which comes last.
- R-217 (amends R-214): the task's size is accepted as it merges (532 counted lines at the head reviewed, plus the fix R-217 requires); a timed-out child's whole process group is killed; the timeout is 300 s provisional.
