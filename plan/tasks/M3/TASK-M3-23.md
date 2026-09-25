# TASK-M3-23 — The occupant-comparison harness

- **Milestone:** M3
- **Closes:** REQ-TOOL-031, REQ-TOOL-032, REQ-TOOL-038, REQ-VAL-053, REQ-INT-019, REQ-INT-074
- **Depends on:** TASK-M3-08, TASK-M3-19
- **Needs (earlier milestones):** REQ-VAL-003
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3, PIT-4.4, PIT-5
- **Size:** ~400 lines

## Goal
`crates/validation` gets the occupant-comparison harness that integrator contract Part 2b makes part of the seam: it refuses arms that differ in stepper or step control, shares fixed colour-ramp constants across arms, prints each arm's full provenance (occupant and profile), and refuses to re-run trajectories with a discretisation different from the run under test. Occupants are never switched automatically; Part 2b defines how the report of which regularisation would have been better is judged, when it runs and where it is reported (R-72).

## References
- `docs/contracts/principia_integrator_contract.md` § "The comparison discipline is part of the seam, NOT a convention"
- `docs/read_first/principia_01_pitfalls.md` § "3. Standing rules earned in this sequence"
- `docs/read_first/principia_01_pitfalls.md` § "4.4 The remedy, and its own falsification test"
- `docs/read_first/principia_00_philosophy.md` § "7.5 Regularisation-method comparison as a scientific result"
- `decisions.md` § "R-74 — The research phases are settled by the vertical slice *(closes RQ-25)*"
- `docs/contracts/principia_integrator_contract.md` § "One thing deliberately NOT decided"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"

## Deliverables
- `crates/validation/src/compare/mod.rs` — `Arm { stepper, regularisation, step_control, discretisation }`, `compare(arms, ics)` returning a typed refusal on mismatch.
- Provenance block per arm; a shared `RampConstants` passed to every panel.
- Doc change: `docs/contracts/principia_integrator_contract.md` § "One thing deliberately NOT decided" — the judging criterion, trigger and report location of the 'which would have been better' report, with no automatic switching (definition, REQ-INT-074).

## Acceptance tests
- `cargo test -p validation compare_refuses_stepper_mismatch` — two arms with different steppers (e.g. GBS logH vs RK4 Heggie) → the harness returns an error, not a result (REQ-TOOL-031).
- Review (physics): comparison output ramp constants identical across panels; each arm's provenance block names stepper, regularisation and full profile (REQ-TOOL-032).
- `cargo test -p validation diagnostic_same_discretisation` — the diagnostic asserts its n_sync, t_max and step parameters equal the run under test's and refuses otherwise; a persistence check with per-window rescaled n_sync is rejected (REQ-TOOL-038).
- `cargo test -p validation logh_heggie_same_arms` — the logH-vs-Heggie harness asserts both arms share the RK4 stepper and step control; a run with a different stepper or step control on one arm is refused (REQ-VAL-053).
- Review (code): no code path selects an occupant from trajectory state; the occupant is a user/sim-key choice (REQ-INT-019).
- Part 2b states the judging criterion, the trigger and the report location, without any automatic switching; physics reviewer approved (REQ-INT-074).

## Notes
- Definitions written here (R-72; physics reviewer approves before merge): REQ-INT-074.
