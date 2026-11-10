# TASK-M8-108 — The Sequence's morph segment

- **Milestone:** M8
- **Closes:** REQ-GUI-421
- **Depends on:** TASK-M2-81, TASK-M8-31, TASK-M8-77, TASK-M8-78, TASK-M8-102, TASK-M8-103, TASK-M8-104, TASK-M8-118, TASK-M8-126, TASK-M7-72
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, perf
- **Pitfalls:** none
- **Size:** ~250 lines

## Goal
A chart-change segment offers morph beside cut and dissolve, with a band on the Chart lane where a cut mark stood. The spec's segment gains its `path` field, `segments: [(easing, transition, path)]` (chart_composition § 17.9), read and written by TASK-M8-31's spec object.

## References
- `docs/design/principia_chart_composition.md` § "17.13 The Sequence"
- `docs/design/principia_sequence.md` § "3.4 Segments: easing per segment, and transitions"
- `docs/design/principia_chart_composition.md` § "17.9 Path shape and timing"

## Deliverables
- `crates/gui/src/`: the segment's control, the band, the planner row.
- Export through CPU batch working from day one for any ends, reading TASK-M2-81's path record (R-916).
- The spec's segment's `path` field (§ 17.9), in TASK-M8-31's spec object: serialised, URL-encoded and read back with the spec; a segment with no path reads as before, so an existing spec reopens bitwise.

## Acceptance tests
- `cargo test -p gui morph_segment` — the band, the row, an old spec bitwise, a link-id change still a cut; controls: a mark on a morph, no flag (REQ-GUI-421).
- `cargo test -p engine spec_segment_path` — a spec whose segment carries a `path` round-trips through the spec's serialisation and its URL encoding bitwise, the path TASK-M2-81's record by value; a spec written before the field reopens bitwise with no path; controls: a path dropped on the round trip fails, an old spec gaining a field on reopen fails (chart_composition § 17.9; no requirement names the field).

## Notes
- **Build order (R-863):** stage 4, morph families, easing and Sequence segments. It depends on stage 3's tasks (TASK-M4-33, TASK-M4-34, TASK-M8-100, TASK-M8-101, TASK-M8-109, TASK-M8-110, TASK-M8-111 and TASK-M8-119; now TASK-M2-71, TASK-M2-72, TASK-M2-74, TASK-M2-75, TASK-M2-76 and TASK-M2-70, TASK-M8-100 retired, its maths TASK-M2-71's) and merges only with their tests green.
- *9 Nov 2026 (R-916):* the line above is a record: none of those is in Depends on now. This task reaches M2's stage-3 tasks through TASK-M2-81, and TASK-M4-33 and TASK-M4-34 through TASK-M8-102.
- What to try: on `cargo run -p gui --features mock`, set a chart-change segment to morph, open the planner and read its row. The default for a new segment is RQ-721's.
- Waits on its rulings item: RQ-721. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); R-917's hold holds it, no longer an edge: R-916 took TASK-M2-58 (was TASK-M4-37), the rulings applied, out of Depends on, and no M3 to M9 task starts until M2's gate passes, which waits on TASK-M2-58 as an M2 task. *Was:* "TASK-M4-37 in Depends on holds it."
- *8 Nov 2026 (R-873):* **Build order:** stage 4; every stage-3 task added to Depends on (RQ-744 (A) (a)). *Was:* "- **Depends on:** TASK-M4-37, TASK-M8-106, TASK-M8-77, TASK-M8-78, TASK-M4-33, TASK-M4-34, TASK-M8-100, TASK-M8-101, TASK-M8-109, TASK-M8-110, TASK-M8-111, TASK-M8-119"
- *9 Nov 2026 (R-916):* the chart core is M2's (R-916, R-917, R-919), so this header follows plan/tasks.yaml. *Was:* "Depends on: TASK-M4-37, TASK-M8-106, TASK-M8-77, TASK-M8-78, TASK-M4-33, TASK-M4-34, TASK-M8-100, TASK-M8-101, TASK-M8-109, TASK-M8-110, TASK-M8-111, TASK-M8-119, TASK-M8-102, TASK-M8-103, TASK-M8-104, TASK-M8-112, TASK-M8-118, TASK-M8-122, TASK-M8-123, TASK-M8-126, TASK-M7-72"; "Reviewers: code, qa, gui".
- *9 Nov 2026 (R-916):* the export arm moves here from TASK-M2-81 (formerly TASK-M8-106), which keeps the path maths in M2; this task gains the perf reviewer for it.
- *9 Nov 2026 (R-916; applied per R-369):* the spec's segment's `path` field is this task's: TASK-M8-31's note gave it to TASK-M8-106, now TASK-M2-81 in M2, which comes before TASK-M8-31 and no longer depends on it; this task took TASK-M2-81's export arm and depends on TASK-M8-31. No requirement names the field, so the acceptance line cites § 17.9.
