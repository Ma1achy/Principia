# TASK-M8-107 — Live paths: the blend entry and the coarse cover

- **Milestone:** M8
- **Closes:** REQ-PERF-117
- **Depends on:** TASK-M4-37, TASK-M8-106
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** none
- **Size:** ~400 lines

## Goal
A path between two baked ends runs live like a tilt: coarse in motion, refining at rest, by one generic entry and no baked pair of charts.

## References
- `docs/design/principia_chart_composition.md` § "17.12 Performance"

## Deliverables
- `crates/kernel/src/stage/`: `stage_blend`.
- The dispatch of three stages per quad; the label for a CPU-only end.

## Acceptance tests
- `cargo test -p engine blend_entry_counted` — stage_blend the only morph entry; control: a baked (A, B) entry (REQ-PERF-117).
- `cargo xtask bench path_cover` — one coarse cover against the design's target; benchmark: waits for the human's Mac and the Windows runner (REQ-PERF-117).

## Notes
- The whole frame, integration included, is measured here for the first time: the design measured only a stand-in for the chart side.
- A faster CPU-only live path is a recorded goal, with no task.
- Waits on its rulings item: RQ-721. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M4-37 in Depends on holds it.
