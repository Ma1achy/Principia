# TASK-M2-75 — Region D's exact map and every target's native coordinates

- **Milestone:** M2
- **Closes:** REQ-CHART-121
- **Depends on:** TASK-M2-58, TASK-M2-68, TASK-M2-65
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~200 lines

## Goal
The read-back map at any masses, and a test per region-D target.

## References
- `docs/design/principia_chart_composition.md` § "8.8 Region D must match the papers (order item A5)"

## Deliverables
- `crates/engine/src/chart_authoring/`: the read-back.
- `crates/engine/tests/region_d_native.rs`.

## Acceptance tests
- `cargo test -p engine region_d_native` — `forward(inverse(x)) = x` on each row, and each row and its control, mutation: a sign flipped in the inverse on one side only (REQ-CHART-121).

## Notes
- **Build order (R-863):** stage 3, the primitive registry and composed axes, each primitive with its round-trip test. It depends on stage 2's task, TASK-M2-65, and merges only with its tests green.
- Waits on its rulings item: RQ-728. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M2-58 in Depends on holds it.
- *9 Nov 2026 (R-916):* moved from M8 to M2 and renumbered: formerly TASK-M8-110; the header follows plan/tasks.yaml. *Was:* "Milestone: M8"; "Depends on: TASK-M4-37, TASK-M4-34, TASK-M8-117".
