# TASK-M8-108 — The Sequence's morph segment

- **Milestone:** M8
- **Closes:** REQ-GUI-421
- **Depends on:** TASK-M4-37, TASK-M8-106, TASK-M8-77, TASK-M8-78
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~250 lines

## Goal
A chart-change segment offers morph beside cut and dissolve, with a band on the Chart lane where a cut mark stood.

## References
- `docs/design/principia_chart_composition.md` § "17.13 The Sequence"
- `docs/design/principia_sequence.md` § "3.4 Segments: easing per segment, and transitions"

## Deliverables
- `crates/gui/src/`: the segment's control, the band, the planner row.

## Acceptance tests
- `cargo test -p gui morph_segment` — the band, the row, an old spec bitwise, a link-id change still a cut; controls: a mark on a morph, no flag (REQ-GUI-421).

## Notes
- What to try: on `cargo run -p gui --features mock`, set a chart-change segment to morph, open the planner and read its row. The default for a new segment is RQ-721's.
- Waits on its rulings item: RQ-721. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M4-37 in Depends on holds it.
