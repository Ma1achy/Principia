# TASK-M2-83 — Time-axis chart mode's maths, through the trajectory interface (R-916, R-918)

- **Milestone:** M2
- **Closes:** REQ-CHART-123, REQ-CHART-174
- **Depends on:** TASK-M2-81, TASK-M2-82, TASK-M2-66, TASK-M2-91
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
- `cargo test -p engine -- path_lock_round_trip` on a time-axis pair — TASK-M2-65's gate test, lock → morph A → B → tilt → slice → morph back, its time-axis arm: the decode at the pin within the lock's tolerance on every frame of the pinned state where the target holds it, and on a sliding pair of the anchor R-849's slide prescribes at that frame (the stored state at s = 0, B's endpoint as REQ-CHART-115's verify reads it: B's nearest point at s = 1 where no cut locus lies on the path, otherwise the followed minimiser, labelled "not nearest" while another point is nearer, and where it ends the last frame where it exists in place of s = 1 (R-935); REQ-CHART-146); the return to `s = 0` bitwise equal to the same tilt and slice applied directly to A about the pin (R1, gating); a time-axis chart to an IC-only chart sliding with no jump, its control the jump (R-950); controls as TASK-M2-65's, the lock dropped on a blend segment, a hidden coordinate invented for the blend, a morph restarted from `s = 0`, each failing (REQ-CHART-161, REQ-CHART-174). *Was (clause, 14 Nov 2026, code 5903884092, physics 5903883598 and qa 5903881780 reviews on PR #528):* "(the stored state at s = 0, B's nearest point at s = 1; REQ-CHART-146)" *Was (clause, 14 Nov 2026, code 5903884092, physics 5903883598 and qa 5903881780 reviews on PR #528):* "a time-axis chart to an IC-only chart keeping its labelled jump;"
- Review: DEFINITION (R-72): § 18.4's tilt and slice on a time-axis chart and G9's time arm written there, approved by the physics reviewer; backed by cargo test -p engine time_anchor and REQ-CHART-161's time-axis pair (REQ-CHART-174).
- *11 Nov 2026 (R-926 item 3, R-950):* `cargo test -p engine time_morph_rules` — as chart_composition § 18.6's dated note has it: a time-to-time morph with different windows runs, its ends exact; time to IC-only morphs, and under a lock no jump stays; control: a cut (REQ-CHART-123).
- *14 Nov 2026, code 5903884092, physics 5903883598 and qa 5903881780 reviews on PR #528 (R-950):* `cargo test -p engine path_lock_centre` on a time-axis chart to an IC-only chart — the time-axis arm of TASK-M2-65's `path_lock_centre`, which TASK-M2-65 can't run before this task builds the time-axis mode: under a lock the anchor slides with no jump, read at the pin, as REQ-CHART-115's verify reads the slide; control: a jump on that pair fails (REQ-CHART-115, R-950).

## Notes
- Split from TASK-M8-113 by R-916 (9 Nov 2026); TASK-M8-113 keeps REQ-CHART-178 (`time_column_matches_playhead`, `time_extend_continues`), which read the real march (F6).
- **Build order (R-916):** stage 5.
- *9 Nov 2026 (Codex P1 on PR #465, applied per R-369):* this task gains the time-axis arm of TASK-M2-65's gate test (`path_lock_round_trip`), which TASK-M2-65, stage 2, can't run before this task, stage 5, builds the time-axis mode; TASK-M2-65's line reads "a time-axis pair once TASK-M2-83 lands". G9 still requires every pair at the gate.
- *11 Nov 2026 (the re-plan's addition (`plan/charts/m2-chart-replan-addition.md`) § F.4; the re-plan's § A.1, CHANGE, and § B.5 item 19):* time is a coordinate (R-926 item 2), a tilt toward time is allowed, and the cut to an IC-only chart is gone; it lands with TASK-M2-91 (time as the last coordinate), a new dependency. `time_morph_rules` changes as its dated line has it (the addition's § B, B10 and B11; V29 retired). Where a dated line under Acceptance tests differs from an earlier line for the same requirement, the dated line binds and the earlier one is kept as its *Was*. *Was (Depends on):* "TASK-M2-81, TASK-M2-82, TASK-M2-66".
