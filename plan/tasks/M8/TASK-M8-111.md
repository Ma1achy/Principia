# TASK-M8-111 — The primitives of the full order

- **Milestone:** M8
- **Closes:** REQ-CHART-119, REQ-CHART-120
- **Depends on:** TASK-M8-109
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~400 lines

## Goal
Each row of the full order's table, as its verdict says, on CPU batch.

## References
- `docs/design/principia_chart_composition.md` § "3.2 The primitives of the full order"

## Deliverables
- `docs/design/principia_chart_reference.md`: the rows.
- `crates/engine/src/chart_authoring/registry.rs`: the rows and aliases.

## Acceptance tests
- `python3 plan/check_plan.py` — the definitions cited (REQ-CHART-119).
- `cargo test -p engine prim_round_trip` and `cargo test -p engine prim_rejected` — the rows of section 3.2 (REQ-CHART-120).

## Notes
- Waits on RQ-723, RQ-725, RQ-726 and RQ-730. Nothing here scores a target; R-668 and R-669 stand.
