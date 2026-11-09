# TASK-M2-83 — Time-axis chart mode's maths, through the trajectory interface (R-916, R-918)

- **Milestone:** M2
- **Closes:** REQ-CHART-123, REQ-CHART-174
- **Depends on:** TASK-M2-81, TASK-M2-82, TASK-M2-66
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~400 lines, the tests included

## Goal
Time-axis chart mode's maths, read through the trajectory interface on its exact fixtures (R-918): the mode, rows on boundaries, the zoom floor, terminals, jitter, the window, morph rules and the anchor, and what a tilt and a slice mean on a time-axis chart (R-72). The column march on the real integrator stays TASK-M8-113's.

## References
- `docs/design/principia_chart_composition.md` § "18.1 Verdict: a chart mode, no fifth axis kind"
- `docs/design/principia_chart_composition.md` § "18.2 Against the core"
- `docs/design/principia_chart_composition.md` § "18.4 Interactions"
- `docs/design/principia_chart_composition.md` § "18.6 Tests"
- `docs/design/principia_chart_composition.md` § "18.3 Stored rows and dense output: the conflict, plainly"
- `docs/design/principia_chart_composition.md` § "18.7 The trajectory interface (R-918, R-919)"
- `decisions.md` § "R-902 — The lock is a property of the path: any locked path, blend segments and chart switches included, keeps the pinned state fixed, sliding per the RQ-721 rider where the target can't hold it; the pivot record is stored as the state; a tilt or slice mid-morph goes to both endpoint charts, each in its own hidden coordinates and tilt map, about the pinned state, the morph continuing from its parameter; the gate test closes stage 2"
- `decisions.md` § "R-916 — M2 is the whole non-visual chart system: the primitive registry, axes from primitives and warps, charts as a pair of axes with their dependence rules, refusals and labels, curved slices, CPU decode and encode through the composition, and the non-visual path maths, each an engine function with its tests, landing before any further downstream chart work, today's charts and view operations re-expressed in it bit for bit first; GPU lowering, contours and display, GUI routes, panels, the Sequence's UI, time-axis mode's UI and overlays keep their milestones as thin adapters onto it; a task mixing maths and visuals is split *(amends R-863; supersedes R-873 (A) (b) and (E) (i); refines R-864 and R-894)*"
- `decisions.md` § "R-918 — R-917's follow-ups: TASK-M3-14 (the Benettin FTLE shadow) stops and is re-queued after M2's gate; M2's maths that seems to need the integrator is tested against exactly known trajectories through a trajectory interface the real integrator plugs into after M2, flagging only what can't be tested so; CI work outside the order, TASK-M2-38 and TASK-M2-56 finishing, and the gate checklist first are agreed *(no RQ)*"

## Deliverables
- `crates/engine/src/chart_authoring/`: `ChartMode::TimeAxis`, its rows read through the trajectory interface.
- The tilt and slice on a time-axis chart written into chart_composition § 18.4 under physics review (R-72; REQ-CHART-174).

## Acceptance tests
- `cargo test -p engine -- time_rows_on_boundaries time_zoom_floor time_terminal_holds time_jitter_ic_only time_two_axes_refused time_window_frozen time_morph_rules time_anchor` — each with its control, on § 18.7's exact fixtures (REQ-CHART-123).
- Review: DEFINITION (R-72): § 18.4's tilt and slice on a time-axis chart and G9's time arm written there, approved by the physics reviewer; backed by cargo test -p engine time_anchor and REQ-CHART-161's time-axis pair (REQ-CHART-174).

## Notes
- Split from TASK-M8-113 by R-916 (9 Nov 2026); TASK-M8-113 keeps REQ-CHART-178 (`time_column_matches_playhead`, `time_extend_continues`), which read the real march (F6).
- **Build order (R-916):** stage 5.
