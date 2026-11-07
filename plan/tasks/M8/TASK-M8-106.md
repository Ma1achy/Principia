# TASK-M8-106 — The path type, the line on blocks, and export through CPU batch

- **Milestone:** M8
- **Closes:** REQ-CHART-113, REQ-CHART-114, REQ-CHART-116, REQ-CHART-124
- **Depends on:** TASK-M4-37, TASK-M8-105, TASK-M8-102, TASK-M8-31
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** none
- **Size:** ~700 lines

## Goal
Every arm of the path type, with tilt and zoom calling their own code, a frame recorded by value, and export working from day one for any ends.

## References
- `docs/design/principia_chart_composition.md` § "17.2 The mechanism"
- `docs/design/principia_chart_composition.md` § "17.3 The constraint: bitwise through the mechanism"
- `docs/design/principia_chart_composition.md` § "17.4 `s = 0` and `s = 1`"
- `docs/design/principia_chart_composition.md` § "17.5 A frame: valid, reproducible, recorded"
- `docs/design/principia_chart_composition.md` § "17.6 Per-axis and pair-map morphs"
- `docs/design/principia_chart_composition.md` § "17.7 Plane to plane"
- `docs/design/principia_chart_composition.md` § "17.9 Path shape and timing"
- `docs/design/principia_chart_composition.md` § "17.10 Without a lock: alignment"
- `docs/design/principia_chart_composition.md` § "17.11 Honesty"
- `docs/design/principia_chart_composition.md` § "17.14 Tests"

## Deliverables
- `crates/engine/src/`: the path type, its arms, the record and the sim key's morph arm.
- CPU batch's path lowering: blocks, the great circle, alignment, re-timing, control charts.
- The planner's row and flag.

## Acceptance tests
- `python3 plan/check_plan.py` — the metric cited (REQ-CHART-124).
- `cargo test -p engine path_tilt_bitwise path_zoom_bitwise path_lock_bitwise path_save_view_bitwise path_keyframe_bitwise path_ends_bitwise path_record_round_trip` — each against the direct operation (REQ-CHART-113).
- `cargo test -p engine path_gram_flags path_blank_ends path_hold_table path_small_step path_links_cut` — each row and its control (REQ-CHART-114).
- `cargo test -p engine path_retime` — spread at most 1.25, pinned ends; control: the table off (REQ-CHART-116).

## Notes
- If path_tilt_bitwise or path_zoom_bitwise can't pass through the path type, the generalisation isn't adopted: stop and report (the owner's rule).
- Waits on its rulings item: RQ-721. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M4-37 in Depends on holds it.
