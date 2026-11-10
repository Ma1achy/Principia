# TASK-M2-74 — The primitives of 6 Nov: axes and overlays

- **Milestone:** M2
- **Closes:** REQ-CHART-117, REQ-CHART-118, REQ-CHART-184
- **Depends on:** TASK-M2-58, TASK-M2-66, TASK-M2-71, TASK-M2-65
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
- Review: the proposal gives the tolerance at f64 and f32 with its criterion, written before the run, and the measured worst over a stated sample (`cargo test -p engine prim_round_trip`); checked by the physics reviewer; confirmed at the M2 gate and recorded in decisions.md (REQ-CHART-184).

## Notes
- **Build order (R-863):** stage 3, the primitive registry and composed axes, each primitive with its round-trip test. It depends on stage 2's task, TASK-M2-65, and merges only with its tests green.
- No row is baked. Nothing here scores a target: R-668 and R-669 stand.
- Waits on its rulings item: RQ-723, RQ-724, RQ-725, RQ-730. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M2-58 in Depends on holds it.
- *7 Nov 2026 (R-847, R-848, R-844):* RQ-723 is ruled (b): inner and outer orbital elements are axes at a labelled pair, plus the overlays; `h` is a builder axis, its range under physics review (RQ-724); Hill's threshold takes the labelled pair (RQ-730 (a)).
- *9 Nov 2026 (R-916):* moved from M8 to M2 and renumbered: formerly TASK-M8-109; the header follows plan/tasks.yaml. *Was:* "Milestone: M8"; "Closes: REQ-CHART-117, REQ-CHART-118"; "Depends on: TASK-M4-37, TASK-M4-33, TASK-M8-100, TASK-M7-72, TASK-M8-117".
- *9 Nov 2026 (R-916):* REQ-CHART-184, the round-trip tolerance § 12 gives only as measured worsts, is this task's calibration, its criterion written first, set by measurement (R-649/650), confirmed at the M2 gate (G5).
