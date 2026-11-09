# TASK-M8-108 — The Sequence's morph segment

- **Milestone:** M8
- **Closes:** REQ-GUI-421
- **Depends on:** TASK-M2-81, TASK-M8-31, TASK-M8-77, TASK-M8-78, TASK-M8-102, TASK-M8-103, TASK-M8-104, TASK-M8-118, TASK-M8-126, TASK-M7-72
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, perf
- **Pitfalls:** none
- **Size:** ~250 lines

## Goal
A chart-change segment offers morph beside cut and dissolve, with a band on the Chart lane where a cut mark stood.

## References
- `docs/design/principia_chart_composition.md` § "17.13 The Sequence"
- `docs/design/principia_sequence.md` § "3.4 Segments: easing per segment, and transitions"

## Deliverables
- `crates/gui/src/`: the segment's control, the band, the planner row.
- Export through CPU batch working from day one for any ends, reading TASK-M2-81's path record (R-916).

## Acceptance tests
- `cargo test -p gui morph_segment` — the band, the row, an old spec bitwise, a link-id change still a cut; controls: a mark on a morph, no flag (REQ-GUI-421).

## Notes
- **Build order (R-863):** stage 4, morph families, easing and Sequence segments. It depends on stage 3's tasks (TASK-M4-33, TASK-M4-34, TASK-M2-71, TASK-M2-72, TASK-M2-74, TASK-M2-75, TASK-M2-76 and TASK-M2-70) and merges only with their tests green.
- What to try: on `cargo run -p gui --features mock`, set a chart-change segment to morph, open the planner and read its row. The default for a new segment is RQ-721's.
- Waits on its rulings item: RQ-721. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); R-917's hold holds it, no longer an edge: R-916 took TASK-M2-58 (was TASK-M4-37), the rulings applied, out of Depends on, and no M3 to M9 task starts until M2's gate passes, which waits on TASK-M2-58 as an M2 task. *Was:* "TASK-M4-37 in Depends on holds it."
- *8 Nov 2026 (R-873):* **Build order:** stage 4; every stage-3 task added to Depends on (RQ-744 (A) (a)). *Was:* "- **Depends on:** TASK-M4-37, TASK-M8-106, TASK-M8-77, TASK-M8-78, TASK-M4-33, TASK-M4-34, TASK-M8-100, TASK-M8-101, TASK-M8-109, TASK-M8-110, TASK-M8-111, TASK-M8-119"
- *9 Nov 2026 (R-916):* the chart core is M2's (R-916, R-917, R-919), so this header follows plan/tasks.yaml. *Was:* "Depends on: TASK-M4-37, TASK-M8-106, TASK-M8-77, TASK-M8-78, TASK-M4-33, TASK-M4-34, TASK-M8-100, TASK-M8-101, TASK-M8-109, TASK-M8-110, TASK-M8-111, TASK-M8-119, TASK-M8-102, TASK-M8-103, TASK-M8-104, TASK-M8-112, TASK-M8-118, TASK-M8-122, TASK-M8-123, TASK-M8-126, TASK-M7-72"; "Reviewers: code, qa, gui".
- *9 Nov 2026 (R-916):* the export arm moves here from TASK-M2-81 (formerly TASK-M8-106), which keeps the path maths in M2; this task gains the perf reviewer for it.
