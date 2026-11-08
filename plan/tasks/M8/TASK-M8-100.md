# TASK-M8-100 — Warps as options, and polar and log-polar pair maps

- **Milestone:** M8
- **Closes:** REQ-CHART-104, REQ-CHART-131, REQ-CHART-132
- **Depends on:** TASK-M4-37, TASK-M4-33, TASK-M3-14
- **Needs (earlier milestones):** REQ-CHART-096
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** none
- **Size:** ~350 lines

## Goal
Power and log as options per axis, polar and log-polar on any pair, each declared extension under R-407 clause 3's review.

## References
- `docs/design/principia_chart_composition.md` § "4. The axis layer: warps"
- `docs/design/principia_chart_composition.md` § "5.1 Pair maps"
- `docs/design/principia_chart_composition.md` § "6.4 Measure: a statistic says what it counts"
- `docs/design/principia_chart_composition.md` § "6.5 Chaos measures live in phase space"
- `docs/contracts/principia_scheduler_contract.md` § "Part 2 — Refinement density is not probability density (enforced here, because this is where it would break)"

## Deliverables
- `crates/kernel/src/chart/paired.rs`: `Paired<C, M>`.
- The warp word on `Mixed`'s uniform.
- The statistic's weight and label: a named measure, or `per pixel of this chart`.

## Acceptance tests
- `cargo test -p kernel warps_and_pair_maps` — `forward(inverse(x)) = x` for each warp and pair map; polar's period and partner; the linear arm bit-for-bit; V1's decode time recorded; controls: a warp not written on an energy axis refused, mutation: named per inverse (REQ-CHART-104).
- `cargo test -p engine statistic_measure` — weighted into a named measure or labelled; a known fraction recovered through a power warp; a paper's own measure equals its flat grid; controls: no weight and no label, the weight inverted, a count of quadtree leaves (REQ-CHART-131).
- `cargo test -p engine ftle_chart_independent` — one bit pattern from a linear chart, a warped one and a third with a different tangent gives one `λ_T`, read on copy 0; control: `δ₀`'s direction from a chart tangent, caught by the third (REQ-CHART-132).

## Notes
- **Build order (R-863):** stage 3, the primitive registry and composed axes, each primitive with its round-trip test. Its edge to stage 2 (TASK-M8-117) isn't made: TASK-M8-117 already waits on it through TASK-M8-106 and TASK-M8-102, so the edge would close a cycle. RQ-744 (A) holds the choice.
- Reviewed under RQ-720's reading of R-594 item 5.
- Waits on its rulings item: RQ-720. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M4-37 in Depends on holds it.
