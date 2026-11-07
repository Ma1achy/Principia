# TASK-M8-112 — Paper frames as presets

- **Milestone:** M8
- **Closes:** REQ-CHART-122
- **Depends on:** TASK-M4-37, TASK-M8-110
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
- `cargo test -p engine preset_paper_coords` — `forward(inverse(x)) = x` on each row, and each row and its control, mutation: a sign flipped in the inverse on one side only (REQ-CHART-122).

## Notes
- RQ-650 is outside TASK-M4-37's gate, so nothing in Depends on holds it: D₁₁'s preset doesn't start until RQ-650 is ruled, by this note. If RQ-729 rules for the presets, TASK-M9-20, -22 and -28 take them, by edges TASK-M4-37 applies then; until it does, those tasks' own functions stand.
- Each preset's base point, centre and window (R-466's definition of a preset) are undefined on these pairs, which aren't two latent coordinate axes: RQ-711, read at TASK-M4-37, or a task that defines them under R-72 as TASK-M9-45 does (design § 20). Only those home values wait: the pairs, the masks and `preset_paper_coords` do not.
- Waits on its rulings item: RQ-729. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M4-37 in Depends on holds it.
