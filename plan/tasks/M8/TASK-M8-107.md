# TASK-M8-107 — Live paths: the blend entry and the coarse cover

- **Milestone:** M8
- **Closes:** REQ-PERF-117
- **Depends on:** TASK-M2-81, TASK-M4-33
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
- `cargo xtask bench path_cover` — one coarse cover against the design's target; benchmark: waits for the human's Mac (R-186), which closes it; the Windows runner's arm is a separate labelled measurement that closes nothing, a named measurement and never a required check (R-711, R-860) (REQ-PERF-117).

## Notes
- **Build order (R-863):** no stage of the owner's order names it, so no stage edge is made; where it goes is RQ-744 (C).
- The whole frame, integration included, is measured here for the first time: the design measured only a stand-in for the chart side.
- A faster CPU-only live path is a recorded goal, with no task.
- Waits on its rulings item: RQ-721. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); R-917's hold holds it, no longer an edge: R-916 took TASK-M2-58 (was TASK-M4-37), the rulings applied, out of Depends on, and no M3 to M9 task starts until M2's gate passes, which waits on TASK-M2-58 as an M2 task. *Was:* "TASK-M4-37 in Depends on holds it."
- *8 Nov 2026 (R-873):* **Build order:** stage 4, through TASK-M2-81, which now depends on every stage-3 task (RQ-744 (A) (a)); R-901: easing is TASK-M2-61's, a property of each path segment, so nothing here adds easing beside it.
- *9 Nov 2026 (R-916):* the chart core is M2's (R-916, R-917, R-919), so this header follows plan/tasks.yaml. *Was:* "Depends on: TASK-M2-58, TASK-M2-81".
