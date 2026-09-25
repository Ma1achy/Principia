# TASK-M6-10 — The integration floor: saturated quads stop refining, flagged floor-limited

- **Milestone:** M6
- **Closes:** REQ-SCHED-063, REQ-SCHED-077
- **Depends on:** TASK-M6-01
- **Needs (earlier milestones):** REQ-REF-006, REQ-TOOL-033, REQ-INT-029
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-2.4, PIT-1.5
- **Size:** ~250 lines

## Goal
A quad whose samples are substep-saturated and whose `suspect_fraction` stays high stops at the integration floor: flagged floor-limited, terminal, never re-queued, while its samples keep their real low-confidence outcomes. The task proposes the `suspect_fraction` threshold and its persistence with its unit (R-71) from traces on substep-saturated near-collision fixtures.

## References
- `docs/contracts/principia_scheduler_contract.md` § "Part 4 — Terminal vs refinable, tied to the two floors"
- `docs/design/principia_deep_zoom.md` § "Reserved: the integration floor"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"

## Deliverables
- `crates/engine/src/refine/integration_floor.rs`: the floor test over `saturated_fraction` and `suspect_fraction` with the persistence counter.
- `fixtures/gates/integration_floor/`: near-collision fixtures; `cargo xtask gate integration-floor` producing the traces.

## Acceptance tests
- `cargo test -p engine integration_floor_stop` (threshold: REQ-SCHED-077 (calibrated)) — fixture quad with saturated samples above the suspect threshold: assert no split, floor-limited flag set, not re-queued, and per-sample outcomes unchanged (REQ-SCHED-063).
- `cargo xtask gate integration-floor` — decisions.md records both with their evidence: suspect_fraction traces on substep-saturated near-collision fixtures, separating floor-limited quads from quads that resolve on refinement; proposal with its evidence in the PR, reviewer-checked; the human confirms the value at the M6 gate and it is recorded in `decisions.md` (REQ-SCHED-077).

## Notes
- Calibration proposed: REQ-SCHED-077.
- PIT-2.4 / PIT-1.5: the floor is a refinement stop, never a sample-terminal — the per-sample outcomes must be asserted unchanged.
