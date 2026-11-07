# TASK-M8-110 — Region D's exact map and every target's native coordinates

- **Milestone:** M8
- **Closes:** REQ-CHART-121
- **Depends on:** TASK-M4-37, TASK-M4-34
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
- `cargo test -p engine region_d_native` — each row and its control (REQ-CHART-121).

## Notes
- Waits on RQ-728.
- Waits on its rulings item: RQ-728. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M4-37 in Depends on holds it.
