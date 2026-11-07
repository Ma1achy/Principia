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
- `cargo test -p engine prim_round_trip` and `cargo test -p engine prim_rejected` — `forward(inverse(x)) = x` on each row with an inverse, at f64 and f32, each row reading back its `Holds`, each with its control, mutation: named per row, firing there (§ 12), the tolerances set here with a stated margin over § 12's measured worsts (REQ-CHART-118).

## Notes
- No row is baked. Nothing here scores a target: R-668 and R-669 stand.
- Waits on its rulings item: RQ-723, RQ-724, RQ-725, RQ-730. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M4-37 in Depends on holds it.
- *7 Nov 2026 (R-847, R-848, R-844):* RQ-723 is ruled (b): inner and outer orbital elements are axes at a labelled pair, plus the overlays; `h` is a builder axis, its range under physics review (RQ-724); Hill's threshold takes the labelled pair (RQ-730 (a)).
