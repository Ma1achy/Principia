# TASK-M4-36 — The virial pair: its axis kind, descriptor and entry

- **Milestone:** M4
- **Closes:** REQ-CHART-103
- **Depends on:** TASK-M4-33, TASK-M2-37
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~150 lines

## Goal
The kernel's `AxisVirial` is built on TASK-M2-37: this gives it the descriptor the validator needs, the registry's row, and a baked `(configuration axis, Q)` pair inside the cap.

## References
- `docs/design/principia_chart_composition.md` § "9. Then virial"
- `docs/design/principia_chart_composition.md` § "3. The primitive registry"

## Deliverables
- `crates/engine/src/chart_authoring/`: `InvariantQuantity`'s or its kind's row, as RQ-717 rules.
- One baked entry.

## Acceptance tests
- `cargo test -p engine virial_descriptor` — the validator takes it; refused beside hold E and a K axis; forward agrees with `virial_ratio`; control: no descriptor refused (REQ-CHART-103).

## Notes
- Waits on RQ-717, and on RQ-700 for a held L_z ≠ 0, where the axis reads above the floor (§ 9). TS-2 stays "not scored: no fixed-k velocity chart" (R-669).
