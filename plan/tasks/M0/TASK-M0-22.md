# TASK-M0-22 — Controls for qa's xtask tests, and `controls` in `cargo xtask ci`

- **Milestone:** M0
- **Closes:** REQ-VAL-007, REQ-VAL-154, REQ-VAL-155, REQ-VAL-156
- **Depends on:** TASK-M0-04, TASK-M0-21, TASK-M0-24, TASK-M0-25
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** PIT-3, PIT-9
- **Size:** counted per R-211 (implementation and the implementer's own tests; qa commits and `negative_control!` blocks excluded): the spawn helper, the macro change and the shared test-support module, est. well under ~500

## Goal
Every test in the workspace has a registered negative control that makes it fail, and `cargo xtask controls` joins `cargo xtask ci`, so from here on a test without a discriminating control turns CI red (philosophy §4.4; pitfalls §3 and §9's general form). Controls for every test merged before it are registered, with TASK-M0-21's `negative_control!(test_name, "description", control)` (R-199), in three parts (R-209): TASK-M0-24 covers the implementer's tests, TASK-M0-25 qa's validation and prin tests, and this task qa's xtask tests (`xtask/tests/qa_TASK-M0-01*.rs`, 33 at R-209), then registers `controls` in `cargo xtask ci` (R-198, R-200).

## References
- `docs/read_first/principia_00_philosophy.md` § "4.4 A test that cannot fail is not a test"
- `docs/read_first/principia_01_pitfalls.md` § "3. Standing rules earned in this sequence"
- `docs/read_first/principia_01_pitfalls.md` § "9. A PARITY CHECK THAT MASKS THE BITS THE FORK LANDS IN"
- `decisions.md` § "R-176 — Controls come before the tests that need them *(closes G3, S2)*"
- `decisions.md` § "R-177 — Cadence *(closes G4, C6)*"
- `decisions.md` § "R-198 — TASK-M0-04 is split into M0-04, M0-21, M0-22 and M0-23 *(closes RQ-135)*"
- `decisions.md` § "R-199 — A test is matched to its control by name in the macro call *(amends R-176; closes RQ-136)*"
- `decisions.md` § "R-200 — TASK-M0-22 is accepted at ~650 lines; TASK-M0-16 depends on it *(closes RQ-137)*"
- `decisions.md` § "R-209 — TASK-M0-22 is split three ways, each part closing its own requirement *(closes RQ-141)*"
- `decisions.md` § "R-211 — TASK-M0-24 and TASK-M0-25 accepted; the size budget counts implementation only *(closes RQ-143, RQ-144)*"
- `decisions.md` § "R-212 — A control names the panic it expects *(amends R-199; closes RQ-145)*"
- `decisions.md` § "R-213 — R-210's exception extends to `qa_r206_harness_opens_the_selected_backend` *(closes RQ-146)*"
- `decisions.md` § "R-214 — Children are spawned through one helper with a timeout *(closes RQ-147)*"
- `decisions.md` § "R-215 — The veto items on PRs #23 and #24 stand; duplicated controls and copied checks are consolidated"

## Deliverables
- The `controls` feature and validation dev-dependency are in place in `xtask` (TASK-M0-24), `prin` (TASK-M0-25) and `validation` (TASK-M0-21) (R-176, R-209).
- A negative control, registered with `negative_control!(test_name, "description", control)` (R-199), for each of qa's tests in `xtask/tests/` (`qa_TASK-M0-01.rs`, `_live`, `_r191`, `_r193`, `_r194`), and for any test merged after R-209's count that TASK-M0-24 and TASK-M0-25 don't cover. Each control is a mutation, a contaminated input, a sign-flipped variant or a comparison that must differ, and makes its test fail. The controls live in new targets where they can; an edit to qa's merged files is limited to what registration forces and is reviewed by qa (R-209).
- `cargo xtask controls` registered in `cargo xtask ci`, so it runs on every push (R-177, R-198), and xtask's ci-registry test updated to match.
- `negative_control!` takes the panic message its control expects and registers it with `#[should_panic(expected = …)]` (R-212; the form in the call is this task's to choose, stated in the PR), and every control registered before this task — TASK-M0-21's, TASK-M0-24's, TASK-M0-25's, qa's and PR #20's — is converted to it.
- The inline controls duplicated by registered ones are removed, and checks copied between test files (the `*_controls.rs` targets TASK-M0-24 and TASK-M0-25 added) move into a shared test-support module that the tests and controls call (R-215). An edit to qa's merged files is limited to replacing a copied check with a call to the module, under R-215's one-round exception, with qa reviewing.
- A shared child-spawn helper in `crates/validation`: it waits at most the timeout (120 s, provisional, REQ-VAL-156) and on timeout kills the child and fails naming it; every test that spawns a child uses it, qa's merged files included, under R-214's one-round exception limited to replacing the spawn call (R-214).
- `qa_r206_harness_opens_the_selected_backend`'s child path moved into the `qa_child` bin, so it is no longer reached through a `#[test]` (R-213, R-210's exception).

## Acceptance tests
- `cargo xtask controls` — on the PR head, every test in each crate declaring the `controls` feature has a registered control and every control makes its test fail; the command runs inside `cargo xtask ci`, in CI on every push; a crate skipped for lacking the feature is listed in its output (REQ-VAL-007).
- Review checklist (qa §3): each control is discriminating, and no test is arithmetically impossible or true by construction (the n_hot < N² quantile case, a distinct-value count bounded below the claimed effect) (REQ-VAL-007).
- `cargo test -p validation spawn` — a child that outlives a short test timeout is killed, the helper's error names it and the killed child no longer exists; a child that exits in time returns its output (REQ-VAL-155).
- `cargo test -p validation -- --list` — lists no child path of `qa_R-206.rs`: `qa_r206_harness_opens_the_selected_backend` is a test only, its child body in the `qa_child` bin (R-213) (REQ-VAL-155).
- `cargo test -p validation --features controls` — a fixture control that panics in its setup fails; with its expected message reaching the check it passes (REQ-VAL-154).
- `cargo xtask controls` — every control in the workspace (validation, prin, xtask) carries an expected message, which the macro requires, and trips it (REQ-VAL-154).
- Proposal (REQ-VAL-156): the helper's 120 s timeout with its evidence (the longest child run measured in the suite, and the headroom), provisional until the human confirms it at the M0 gate.
- Review checklist (code and qa): no inline control duplicates a registered one, no check body is copied between test files, no test spawns a child except through the helper (REQ-VAL-154, REQ-VAL-155).

## Notes
- R-209 (closes RQ-141): at R-209, 113 tests needed controls (~1,300 lines), so the task is split three ways and the two earlier parts close REQ-VAL-152 and REQ-VAL-153. This task, the last, still closes REQ-VAL-007, and every task that depends on it still waits for all of it.
- R-200 (closes RQ-137): at ~650 lines this task is over one reviewable PR (`plan/WORKFLOW.md` § "Task files"), and a further split would leave a part that closes no requirement; the human accepted it at that size, as repetitive registration of one control per test, as R-188 did for TASK-M0-01. TASK-M0-16 depends on this task, so TASK-M0-16 to TASK-M0-18 register their own controls and this task covers only TASK-M0-01's, TASK-M0-04's and TASK-M0-21's tests.
- Every later task's tests register their controls in that task, and the qa reviewer checks each control is discriminating (qa §3). Tasks whose tests would otherwise merge between TASK-M0-04 and this one now depend on it (TASK-M0-02, -03 by R-198; TASK-M0-05, -06, -07 and -20 by R-176's "controls come before the tests that need them"; TASK-M0-16 by R-200).
- A GPU test's control runs where the test does, in `gpu-metal` and `gpu-lavapipe` (TASK-M0-04). On a runner with no adapter the test would fail under its control for the wrong reason, which shows nothing (philosophy §4.4).
- R-211 to R-215 (closes RQ-143 to RQ-147): the size budget counts implementation only; controls name their expected panic; the R-206 child path leaves libtest; children are spawned with a timeout; duplicated inline controls and copied checks are consolidated. All land in this task.
