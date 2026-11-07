# TASK-M8-109 — The primitives of 6 Nov: axes and overlays

- **Milestone:** M8
- **Closes:** REQ-CHART-117, REQ-CHART-118
- **Depends on:** TASK-M4-37, TASK-M4-33, TASK-M8-100, TASK-M7-72
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~400 lines

## Goal
Every row of the registry's 6 Nov table, each as an axis on CPU batch, an alias or a forward-only overlay, as its verdict says.

## References
- `docs/design/principia_chart_composition.md` § "3.1 The primitives of 6 Nov"
- `docs/design/principia_chart_composition.md` § "5.5 Every case where two axes can't be independent"
- `docs/design/principia_chart_composition.md` § "6.1 Refusals: genuine contradictions only"

## Deliverables
- `docs/design/principia_chart_reference.md`: the rows.
- `crates/engine/src/chart_authoring/registry.rs`: the rows and aliases.
- The pair table's rows 17 to 22.

## Acceptance tests
- `python3 plan/check_plan.py` — the definitions cited (REQ-CHART-117).
- `cargo test -p engine prim_round_trip` and `cargo test -p engine prim_rejected` — each row and its control (REQ-CHART-118).

## Notes
- Waits on RQ-723, RQ-724 and RQ-725. No row is baked. Nothing here scores a target: R-668 and R-669 stand.
- Waits on its rulings item: RQ-723, RQ-724, RQ-725, RQ-730. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M4-37 in Depends on holds it.
