# TASK-M2-76 — The primitives of the full order

- **Milestone:** M2
- **Closes:** REQ-CHART-119, REQ-CHART-120, REQ-CHART-185
- **Depends on:** TASK-M2-58, TASK-M2-74, TASK-M2-65
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
- `cargo test -p engine prim_round_trip` and `cargo test -p engine prim_rejected` — `forward(inverse(x)) = x` on the rows of section 3.2 with an inverse, each reading back its `Holds`, mutation: named per row, firing there (§ 12); and Tanikawa–Mikkola's mass-weighted `L_com = 0` at random masses after `C`, from an independent `Σ m r × v`; controls: `R` from the centroid, an unweighted sum; the tolerances set here with a stated margin over § 12's measured worsts (REQ-CHART-120).
- Review: the proposal gives the tolerance at f64 and f32 with its criterion, written before the run, and the measured worst over a stated sample (`cargo test -p engine prim_round_trip`); checked by the physics reviewer; confirmed at the M2 gate and recorded in decisions.md (REQ-CHART-185).

## Notes
- **Build order (R-863):** stage 3, the primitive registry and composed axes, each primitive with its round-trip test. It depends on stage 2's task, TASK-M2-65, and merges only with its tests green.
- Nothing here scores a target; R-668 and R-669 stand.
- Waits on its rulings item: RQ-723, RQ-725, RQ-726, RQ-730. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M2-58 in Depends on holds it.
- *7 Nov 2026 (R-847):* RQ-725 and RQ-726 are ruled (b), R-669 reversed: the scattering rows and fixed-`k` are axes, and their charts are TASK-M2-78's and TASK-M2-79's.
- *9 Nov 2026 (R-916):* moved from M8 to M2 and renumbered: formerly TASK-M8-111; the header follows plan/tasks.yaml. *Was:* "Milestone: M8"; "Closes: REQ-CHART-119, REQ-CHART-120".
- *9 Nov 2026 (R-916):* REQ-CHART-185, the round-trip tolerance § 12 gives only as measured worsts, is this task's calibration, its criterion written first, set by measurement (R-649/650), confirmed at the M2 gate (G5).
