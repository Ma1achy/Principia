# TASK-M8-119 — The virial pair: its axis kind, descriptor and entry

- **Milestone:** M8
- **Closes:** REQ-CHART-103
- **Depends on:** TASK-M4-37, TASK-M4-33, TASK-M2-37
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
- `cargo test -p engine virial_descriptor` — the validator takes it; refused beside hold E and a K axis; forward by mapping: linear agrees with `virial_ratio`, warped at a held L_z ≠ 0 reads `2(K − L_z²/2I)/|U|` and round-trips, labelled `Q above the floor`; control: no descriptor refused, the warped row read as the state's ratio (REQ-CHART-103).

## Notes
- RQ-700 is outside TASK-M4-37's gate, so nothing in Depends on holds it: the arm for a held L_z ≠ 0, where the axis reads above the floor (§ 9), doesn't start until RQ-700 is ruled, by this note. TS-2 stays "not scored: no fixed-k velocity chart" (R-669).
- Waits on its rulings item: RQ-717. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M4-37 in Depends on holds it.
