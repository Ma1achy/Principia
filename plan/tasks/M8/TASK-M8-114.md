# TASK-M8-114 — Time-axis storage and the column cost

- **Milestone:** M8
- **Closes:** REQ-PERF-118
- **Depends on:** TASK-M4-37, TASK-M8-113
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, perf
- **Pitfalls:** none
- **Size:** ~250 lines

## Goal
Stored rows under the budget, and the GPU column measured.

## References
- `docs/design/principia_chart_composition.md` § "18.3 Stored rows and dense output: the conflict, plainly"
- `docs/design/principia_chart_composition.md` § "18.5 Cost per column"

## Deliverables
- `crates/engine/src/`: the row store and its ceiling.
- `xtask` bench `time_column`.

## Acceptance tests
- `cargo test -p engine time_memory_budget` (REQ-PERF-118).
- `cargo xtask bench time_column` — waits for the human's Mac (R-186), which closes it; the Windows runner's arm is a separate labelled measurement that closes nothing (R-711) (REQ-PERF-118).

## Notes
- The column entry counts in the cap's C2. The re-measure runs on the default occupant, at Δ_sync's row spacing; REQ-INT-080's value bears on this mode.
- Waits on its rulings item: RQ-727. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M4-37 in Depends on holds it.
