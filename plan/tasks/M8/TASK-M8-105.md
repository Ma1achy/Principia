# TASK-M8-105 — Chart paths in the contracts

- **Milestone:** M8
- **Closes:** REQ-CHART-112
- **Depends on:** TASK-M4-37, TASK-M4-32
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~250 lines

## Goal
RQ-721's ruling written into the contracts, docs first, so that the build tasks cite contracts and not this design.

## References
- `docs/design/principia_chart_composition.md` § "17.2 The mechanism"
- `docs/design/principia_chart_composition.md` § "17.6 Per-axis and pair-map morphs"
- `docs/design/principia_chart_composition.md` § "17.7 Plane to plane"
- `docs/design/principia_chart_composition.md` § "17.9 Path shape and timing"
- `docs/design/principia_chart_composition.md` § "17.13 The Sequence"
- `docs/contracts/principia_export_animation_contract.md` § "Part 3 — The keyframe system (retained)"

## Deliverables
- `docs/contracts/principia_export_animation_contract.md`: the transition, the path, the frame record.
- `docs/design/principia_sequence.md` and `docs/gui/principia_render_gui_spec.md`: the morph segment.
- REQ-TOOL-075, REQ-GUI-374, REQ-GUI-376, REQ-GUI-379 and REQ-GUI-381 through reqio, as ruled.

## Acceptance tests
- `python3 plan/check_plan.py` — the plan passes with the definition cited (REQ-CHART-112).

## Notes
- Waits on RQ-721.
- Waits on its rulings item: RQ-721. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M4-37 in Depends on holds it.
