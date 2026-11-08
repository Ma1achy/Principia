# TASK-M8-111 — The primitives of the full order

- **Milestone:** M8
- **Closes:** REQ-CHART-119, REQ-CHART-120
- **Depends on:** TASK-M4-37, TASK-M8-109
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

## Notes
- Nothing here scores a target; R-668 and R-669 stand.
- Waits on its rulings item: RQ-723, RQ-725, RQ-726, RQ-730. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M4-37 in Depends on holds it.
- *7 Nov 2026 (R-847):* RQ-725 and RQ-726 are ruled (b), R-669 reversed: the scattering rows and fixed-`k` are axes, and their charts are TASK-M8-122's and TASK-M8-123's.
