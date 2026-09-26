# TASK-M0-22 — Controls for every merged test, and `controls` in `cargo xtask ci`

- **Milestone:** M0
- **Closes:** REQ-VAL-007
- **Depends on:** TASK-M0-04, TASK-M0-21
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** PIT-3, PIT-9
- **Size:** ~650 lines (over the ~500 of one reviewable PR: RQ-137)

## Goal
Every test in the workspace has a registered negative control that makes it fail, and `cargo xtask controls` joins `cargo xtask ci`, so from here on a test without a discriminating control turns CI red (philosophy §4.4; pitfalls §3 and §9's general form). This task registers, with TASK-M0-21's `negative_control!(test_name, "description", control)` (R-199), a control for every test merged before it: TASK-M0-01's, TASK-M0-04's and TASK-M0-21's, and those of any other task merged before this one (R-198).

## References
- `docs/read_first/principia_00_philosophy.md` § "4.4 A test that cannot fail is not a test"
- `docs/read_first/principia_01_pitfalls.md` § "3. Standing rules earned in this sequence"
- `docs/read_first/principia_01_pitfalls.md` § "9. A PARITY CHECK THAT MASKS THE BITS THE FORK LANDS IN"
- `decisions.md` § "R-176 — Controls come before the tests that need them *(closes G3, S2)*"
- `decisions.md` § "R-177 — Cadence *(closes G4, C6)*"
- `decisions.md` § "R-198 — TASK-M0-04 is split into M0-04, M0-21, M0-22 and M0-23 *(closes RQ-135)*"
- `decisions.md` § "R-199 — A test is matched to its control by name in the macro call *(amends R-176; closes RQ-136)*"

## Deliverables
- The `controls` feature and a dev-dependency on `crates/validation` in each crate holding a test merged before this task: `xtask` and `prin` (TASK-M0-01), `validation` (TASK-M0-04), and any other crate whose tests merged before this one (R-176).
- A negative control, registered with `negative_control!(test_name, "description", control)` (R-199), for every test merged before this task: TASK-M0-01's tests in `xtask/tests/` (80 at R-198) and `crates/prin/tests/` (2); TASK-M0-04's `gpu_harness`, `gpu_harness_can_fire`, `metal_hosted_probe`, `gpu_backend_env` and `prop_seed` tests; TASK-M0-21's `controls` tests; and the tests of any other task merged before this one (R-198). Each control is a mutation, a contaminated input, a sign-flipped variant or a comparison that must differ, and makes its test fail.
- `cargo xtask controls` registered in `cargo xtask ci`, so it runs on every push (R-177, R-198).

## Acceptance tests
- `cargo xtask controls` — on the PR head, every test in each crate declaring the `controls` feature has a registered control and every control makes its test fail; the command runs inside `cargo xtask ci`, in CI on every push; a crate skipped for lacking the feature is listed in its output (REQ-VAL-007).
- Review checklist (qa §3): each control is discriminating, and no test is arithmetically impossible or true by construction (the n_hot < N² quantile case, a distinct-value count bounded below the claimed effect) (REQ-VAL-007).

## Notes
- **Blocked on RQ-137:** at ~650 lines this task is over one reviewable PR (`plan/WORKFLOW.md` § "Task files"), and a further split would leave a part that closes no requirement. It waits for the human's ruling.
- Every later task's tests register their controls in that task, and the qa reviewer checks each control is discriminating (qa §3). Tasks whose tests would otherwise merge between TASK-M0-04 and this one now depend on it (TASK-M0-02, -03 by R-198; TASK-M0-05, -06, -07 and -20 by R-176's "controls come before the tests that need them").
- A GPU test's control runs where the test does, in `gpu-metal` and `gpu-lavapipe` (TASK-M0-04). On a runner with no adapter the test would fail under its control for the wrong reason, which shows nothing (philosophy §4.4).
