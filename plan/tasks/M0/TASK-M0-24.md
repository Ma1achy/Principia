# TASK-M0-24 — Controls for the implementer's merged tests, and xtask's `controls` feature

- **Milestone:** M0
- **Closes:** REQ-VAL-152
- **Depends on:** TASK-M0-04, TASK-M0-21
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** PIT-3
- **Size:** ~450–500 lines

## Goal
The first of R-209's three parts of TASK-M0-22. Every test the implementer merged before TASK-M0-22 gets a registered negative control that makes it fail (philosophy §4.4), with TASK-M0-21's `negative_control!(test_name, "description", control)` (R-199). xtask declares the `controls` feature, and the one doctest in a controls crate stops being a test (R-208).

## References
- `docs/read_first/principia_00_philosophy.md` § "4.4 A test that cannot fail is not a test"
- `decisions.md` § "R-176 — Controls come before the tests that need them *(closes G3, S2)*"
- `decisions.md` § "R-199 — A test is matched to its control by name in the macro call *(amends R-176; closes RQ-136)*"
- `decisions.md` § "R-201 — A kernel or ledger unit test's control is registered from that crate's `tests/`, by name *(closes RQ-138)*"
- `decisions.md` § "R-208 — TASK-M0-21 is accepted at 762 code lines; later overruns are split first"
- `decisions.md` § "R-209 — TASK-M0-22 is split three ways, each part closing its own requirement *(closes RQ-141)*"

## Deliverables
- `xtask/Cargo.toml` — the `controls` feature and a dev-dependency on `crates/validation` (R-176).
- A negative control, registered with `negative_control!` (R-199), for each of the implementer's tests merged before TASK-M0-22: validation's `gpu::tests` and `prop::tests` unit tests (registered from `crates/validation/tests/`, R-201), and xtask's `tests/ci.rs`, `tests/controls.rs` and `tests/deps.rs` (55 at R-209). Each control is a mutation, a contaminated input, a sign-flipped variant or a comparison that must differ, and makes its test fail. Where a test already carries an inline control, the registered one replaces or joins it, as the PR states.
- The example in `crates/validation/src/control.rs` made a non-test block (for example `text`), since a doctest in a controls crate counts as a test without a control (R-208).
- `controls_on_this_workspace_skips_gui` (`xtask/tests/controls.rs`) runs the command on the live workspace; once xtask declares the feature, its control must not recurse into xtask's own controls (for example it runs against a fixture).

## Acceptance tests
- `cargo xtask controls` — on the PR head, each test listed above is paired with a control that makes it fail, and validation lists no doctest; the command may still fail naming qa's tests, which TASK-M0-25 and TASK-M0-22 cover (REQ-VAL-152).
- Review checklist (qa §3): each control is discriminating (REQ-VAL-152).

## Notes
- R-209 (closes RQ-141): TASK-M0-22 is split into this task, TASK-M0-25 and TASK-M0-22 itself, which comes last and registers `controls` in `cargo xtask ci`.
- A GPU test's control runs where the test does, in `gpu-metal` and `gpu-lavapipe` (TASK-M0-04).
