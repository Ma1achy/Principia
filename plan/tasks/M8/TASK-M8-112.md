# TASK-M8-112 — Paper frames as presets

- **Milestone:** M8
- **Closes:** REQ-CHART-122
- **Depends on:** TASK-M8-110
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~250 lines

## Goal
Each paper's frame as a preset over the registry, tested against its coordinates.

## References
- `docs/design/principia_chart_composition.md` § "20. Paper frames as presets (order part E)"

## Deliverables
- `crates/engine/src/chart_authoring/presets.rs`: the frames.
- `crates/engine/tests/preset_paper_coords.rs`.

## Acceptance tests
- `cargo test -p engine preset_paper_coords` — each row and its control (REQ-CHART-122).

## Notes
- Waits on RQ-729 and RQ-650. The M9 tasks take these presets.
