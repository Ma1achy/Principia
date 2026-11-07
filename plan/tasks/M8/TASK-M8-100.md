# TASK-M8-100 — Warps as options, and polar and log-polar pair maps

- **Milestone:** M8
- **Closes:** REQ-CHART-104
- **Depends on:** TASK-M4-37, TASK-M4-33
- **Needs (earlier milestones):** REQ-CHART-096
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** none
- **Size:** ~350 lines

## Goal
Power and log as options per axis, polar and log-polar on any pair, each declared extension under R-407 clause 3's review.

## References
- `docs/design/principia_chart_composition.md` § "4. The axis layer: warps"
- `docs/design/principia_chart_composition.md` § "5.1 Pair maps"

## Deliverables
- `crates/kernel/src/chart/paired.rs`: `Paired<C, M>`.
- The warp word on `Mixed`'s uniform.

## Acceptance tests
- `cargo test -p kernel warps_and_pair_maps` — each warp's inverse; polar's period and partner; the linear arm bit-for-bit; V1's decode time recorded; control: a warp not written on an energy axis refused (REQ-CHART-104).

## Notes
- Reviewed under RQ-720's reading of R-594 item 5.
- Waits on its rulings item: RQ-720. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M4-37 in Depends on holds it.
