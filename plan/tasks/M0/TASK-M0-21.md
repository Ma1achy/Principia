# TASK-M0-21 — The negative-control registry and `cargo xtask controls`

- **Milestone:** M0
- **Closes:** REQ-VAL-147
- **Depends on:** TASK-M0-01
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~470 lines

## Goal
Every test registers the discriminating control that must make it fail — a mutation, a contaminated input, a sign-flipped variant or a comparison that must differ — so that no test in the suite is one that cannot fail (philosophy §4.4; pitfalls §9's general form). This task builds the mechanism: `negative_control!(test_name, "description", control)` in `crates/validation`, which names its test in the macro call (R-199), and `cargo xtask controls`, which pairs tests and controls by that name across a crate's targets (R-201), runs every control and fails if a control leaves its test passing or a test has none. It is not yet registered in `cargo xtask ci`: the tests already merged have no controls until TASK-M0-22 (R-198).

## References
- `docs/read_first/principia_00_philosophy.md` § "4.4 A test that cannot fail is not a test"
- `docs/read_first/principia_01_pitfalls.md` § "9. A PARITY CHECK THAT MASKS THE BITS THE FORK LANDS IN"
- `decisions.md` § "R-176 — Controls come before the tests that need them *(closes G3, S2)*"
- `decisions.md` § "R-187 — kernel and ledger may take validation as a dev-dependency; validation never depends on prin *(closes RQ-129)*"
- `decisions.md` § "R-198 — TASK-M0-04 is split into M0-04, M0-21, M0-22 and M0-23 *(closes RQ-135)*"
- `decisions.md` § "R-199 — A test is matched to its control by name in the macro call *(amends R-176; closes RQ-136)*"
- `decisions.md` § "R-201 — A kernel or ledger unit test's control is registered from that crate's `tests/`, by name *(closes RQ-138)*"

## Deliverables
- `crates/validation/src/control.rs` — the negative-control registry: `negative_control!(test_name, "description", control)`, which names its test in the call (no test-name attribute and no proc-macro crate, R-199), and a `controls` feature under which each registered control runs its test against the control input. Crates reach the macro through a dev-dependency on `crates/validation` (R-176).
- `xtask/src/controls.rs` — `cargo xtask controls`: lists the workspace's tests, runs `cargo test --features controls` in each crate that declares the feature (a crate without it is skipped and reported, not failed, R-176), pairs each test with its control by the name in the macro call (R-199) across all of a crate's test targets, so that a control registered in the crate's `tests/` pairs with a unit test in its `src/` (R-201), and fails naming the test when a control leaves its test passing or a test in a controls crate has no control. Not registered in `cargo xtask ci` (TASK-M0-22 registers it, R-198).
- `xtask/tests/fixtures/controls/` — fixture crates: one test with a discriminating control; one test with no control; one test whose control leaves it passing; one crate without the `controls` feature; one crate with a unit test in `src/` whose control is registered in its `tests/` (R-201).
- `xtask/tests/controls.rs` — runs `cargo xtask controls` over each fixture and asserts its verdict.

## Acceptance tests
- `cargo test -p xtask controls` — on the fixtures: the test with a discriminating control passes the command; the test with no control and the test whose control leaves it passing each fail it, naming the test; the crate without the `controls` feature is reported as skipped and does not fail it; the pairing is by the name given in `negative_control!` (REQ-VAL-147).
- `cargo test -p xtask controls` — on the cross-target fixture: the unit test in `src/` is paired by name with the control registered in the same crate's `tests/`, is not reported as lacking a control, and passes the command; with that control removed, the command fails naming the unit test (R-201) (REQ-VAL-147).
- Review checklist (code): no proc-macro crate and no test-name attribute is added; no workspace edge outside systems_architecture §7.1 is added, and `cargo xtask deps` stays green (REQ-VAL-147).

## Notes
- This task's own tests register their controls in TASK-M0-22, with every other test merged before it (R-198). Its fixtures are themselves the command's discriminating cases: the command must go red on two of them.
- `kernel` and `ledger` reach `validation` only as a dev-dependency, and only from integration tests; `gui` has no route to it at all (R-187). A crate without the `controls` feature is skipped, not failed. A unit test in `kernel` or `ledger` has its control registered from the same crate's integration tests (`tests/`), paired by the test's name, and `cargo xtask controls` pairs across a crate's targets (R-201, closing RQ-138); the cross-target fixture shows it.
- R-199 amends R-176: "Tests are matched to their controls by a shared test-name attribute" is replaced by the name in the macro call; the dev-dependency route and the skipping of crates without the feature stand.
