# TASK-M8-114 — Time-axis storage and the column cost

- **Milestone:** M8
- **Closes:** REQ-PERF-118
- **Depends on:** TASK-M4-37, TASK-M8-113
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics, perf
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
- `cargo test -p engine time_memory_budget` — after any order of § 18.3's events, at most K entries, every required boundary held or arriving (REQ-PERF-118).
- `cargo test -p engine -- time_column_matches_playhead time_extend_continues` — the store arms of § 18.6, with their controls (REQ-PERF-118).
- `cargo xtask bench time_column` — waits for the human's Mac (R-186), which closes it; the Windows runner's arm is a separate labelled measurement that closes nothing, a named measurement and never a required check (R-711, R-860) (REQ-PERF-118).

## Notes
- The column entry counts in the cap's C2. The re-measure runs on the default occupant, at Δ_sync's row spacing; REQ-INT-080's value bears on this mode.
- Waits on its rulings item: RQ-727. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M4-37 in Depends on holds it.
