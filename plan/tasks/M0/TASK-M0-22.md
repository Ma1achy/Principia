# TASK-M0-22 — Controls for qa's xtask tests, and `controls` in `cargo xtask ci`

- **Milestone:** M0
- **Closes:** REQ-VAL-007
- **Depends on:** TASK-M0-04, TASK-M0-21, TASK-M0-24, TASK-M0-25, TASK-M0-26, TASK-M0-27, TASK-M0-28, TASK-M0-29, TASK-M0-30
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** PIT-3, PIT-9
- **Size:** ~60 counted lines per R-211 (part (c) of RQ-149's split, R-221); R-216 had put it at ~285 with parts 3 and 5 of RQ-148's split, before RQ-149 counted the helper moves

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
- `decisions.md` § "R-216 — TASK-M0-22 is split three ways under R-211 *(closes RQ-148)*"
- `decisions.md` § "R-218 — Inputs a control shares with its test live in the shared module too *(extends R-215)*"
- `decisions.md` § "R-221 — TASK-M0-22 is split four ways under R-208 *(closes RQ-149)*"

## Deliverables
- The `controls` feature and validation dev-dependency are in place in `xtask` (TASK-M0-24), `prin` (TASK-M0-25) and `validation` (TASK-M0-21) (R-176, R-209).
- A negative control, registered with `negative_control!` in the form TASK-M0-26 gives it, naming the panic it expects (R-199, R-212), for each of qa's tests in `xtask/tests/` (`qa_TASK-M0-01.rs`, `_live`, `_r191`, `_r193`, `_r194`), and for any test merged after R-209's count that TASK-M0-24 and TASK-M0-25 don't cover. Each control is a mutation, a contaminated input, a sign-flipped variant or a comparison that must differ, and makes its test fail. The controls live in new targets where they can; an edit to qa's merged files is limited to what registration forces and is reviewed by qa (R-209).
- `cargo xtask controls` registered in `cargo xtask ci`, so it runs on every push (R-177, R-198), and xtask's ci-registry test updated to match.
- The controls call the checks and helpers TASK-M0-29 and TASK-M0-30 put in shared test-support modules; they copy none (R-215, R-218, R-221).

## Acceptance tests
- `cargo xtask controls` — on the PR head, every test in each crate declaring the `controls` feature has a registered control and every control makes its test fail; the command runs inside `cargo xtask ci`, in CI on every push; a crate skipped for lacking the feature is listed in its output (REQ-VAL-007).
- Review checklist (qa §3): each control is discriminating, and no test is arithmetically impossible or true by construction (the n_hot < N² quantile case, a distinct-value count bounded below the claimed effect) (REQ-VAL-007).

## Notes
- R-209 (closes RQ-141): at R-209, 113 tests needed controls (~1,300 lines), so the task is split three ways and the two earlier parts close REQ-VAL-152 and REQ-VAL-153. This task, the last, still closes REQ-VAL-007, and every task that depends on it still waits for all of it.
- R-200 (closes RQ-137): at ~650 lines this task is over one reviewable PR (`plan/WORKFLOW.md` § "Task files"), and a further split would leave a part that closes no requirement; the human accepted it at that size, as repetitive registration of one control per test, as R-188 did for TASK-M0-01. TASK-M0-16 depends on this task, so TASK-M0-16 to TASK-M0-18 register their own controls and this task covers only TASK-M0-01's, TASK-M0-04's and TASK-M0-21's tests.
- Every later task's tests register their controls in that task, and the qa reviewer checks each control is discriminating (qa §3). Tasks whose tests would otherwise merge between TASK-M0-04 and this one now depend on it (TASK-M0-02, -03 by R-198; TASK-M0-05, -06, -07 and -20 by R-176's "controls come before the tests that need them"; TASK-M0-16 by R-200).
- A GPU test's control runs where the test does, in `gpu-metal` and `gpu-lavapipe` (TASK-M0-04). On a runner with no adapter the test would fail under its control for the wrong reason, which shows nothing (philosophy §4.4).
- R-211 to R-215 (closes RQ-143 to RQ-147): the size budget counts implementation only; controls name their expected panic; the R-206 child path leaves libtest; children are spawned with a timeout; duplicated inline controls and copied checks are consolidated.
- R-216 (closes RQ-148): those land in three tasks — TASK-M0-26 (spawn helper, R-213's move, expected messages), TASK-M0-27 (qa's copied checks into shared modules), and this one (the implementer's consolidation, qa's xtask controls, `controls` in ci).
- R-218: the shared-module rule covers inputs a control shares with its test, not only checks; the shader text in
  `qa_TASK-M0-04_controls.rs` moves in TASK-M0-28 (R-221).
- R-221 (closes RQ-149): qa's xtask helpers (~710 lines) and inline checks (~200) must move into support modules before
  a control in a new target can trip the test's own assertion, which RQ-148 didn't count. The task is split four ways:
  TASK-M0-28 (REQ-VAL-158, REQ-VAL-159), TASK-M0-29 (`qa_TASK-M0-01.rs` and `_live`, REQ-VAL-160), TASK-M0-30 (`_r191`,
  `_r193`, `_r194`, REQ-VAL-161), and this one (the 33 controls and `controls` in ci, REQ-VAL-007).
- Not yet measured (RQ-148): qa's `qa_cargo_xtask_alias_runs_deps` (`xtask/tests/qa_TASK-M0-01.rs:489-520`) runs `cargo xtask ci` as a child; once `controls` joins `ci`, that child builds with every `controls` feature in a fresh target directory and runs every control, under R-214's helper (300 s provisional, R-217; the estimate at TASK-M0-26 was ~102 s cold, ~25 s warm). If it does not fit, that is a conflict between R-214 and R-198, and goes to REVIEW_QUEUE with the measured time.
- Measured (RQ-149, with `controls` in `ci` as a scratch change): 60.6 s cold (61.8 s with its binary's 13 tests in
  parallel), 28.7 s warm; the 16 r191/r193/r194 controls still to be written add an estimated 10–20 s. It fits under
  300 s. Re-measure on the PR.
