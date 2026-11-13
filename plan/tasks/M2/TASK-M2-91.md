# TASK-M2-91 — Time as the last coordinate: the map's time coordinate, the playhead the window's centre, a target time to an integer index (R-926 item 2)

- **Milestone:** M2
- **Closes:** REQ-CHART-191
- **Depends on:** TASK-M2-82, TASK-M2-61
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** kernel and engine; no split named

## Goal
Time is the map's last coordinate (R-926 item 2): the playhead is the window's centre along it, and a target time maps to an integer index on the integrator's grid (R-935, under RQ-761 (a) and (a-i): a per-sample stop index, the label "reached its time", the zoom along time floored at one boundary). It lands with TASK-M2-83 (time-axis chart mode's maths), which depends on it.

## References
- `decisions.md` § "R-926 — The new chart system: a chart is a map from named coordinates to a system state and a time, composed with a flat window of a centre and a basis, every view edit an edit of the window and an axis change a tilt to 90°; time the last coordinate of every map, the playhead the window's centre along it; a morph a blend of two charts' states, linear in masses, momenta and time and a slerp on shape, exact at both ends, navigated by one pixel-space edit on both ends; the lock one constraint, "this pixel shows this system", re-solved; the CPU evaluates charts in f64 and the GPU only integrates records, with no chart code in the integrator; per-quad forward tables and no inverse on that path; a backwards table with a table search fallback for the fragment's needs only; no refusal path; three slope tiers; table debug views; a small quad's one-cell table the linearised decoder *(supersedes R-839; amends R-842; refines R-902)*"
- `decisions.md` § "R-935 — The chart re-plan's one-way questions, the plan's recommendation on each: exact or implicit slopes are the linearised decoder's `J_D`, central differences only the check (RQ-758); the lock's re-solve follows its local minimiser continuously, stops where that ends, and keeps the offered re-centre, the navigation stop and the stored-words select (RQ-759); a per-sample stop index on the integrator's grid with a new label "reached its time", the zoom along time floored at one boundary (RQ-761); the window's scale is R-507's and R-920's unit, time's the home time window (RQ-762); stage 1 measures each operation against its generic edit first (RQ-763); the GPU decode entries stay as test entries and R-837's cap is retired once measured (RQ-765); a large quad's forward table has one exact node per sample, R-90's trigger read on the narrowed f32 records (RQ-766); the table seeds a refinement on the chart's forward code (RQ-767); `ctx.chart.z` stays the forward value, carried in the table (RQ-768); stage 3 runs beside stages 1 and 2 (RQ-769); an in-plane pan or zoom still re-addresses the cache (RQ-770) *(closes RQ-758, RQ-759, RQ-761, RQ-762, RQ-763, RQ-765, RQ-766, RQ-767, RQ-768, RQ-769 and RQ-770; amends R-837, R-849, R-863 and R-864; refines R-90 and R-926)*"
- `decisions.md` § "R-918 — R-917's follow-ups: TASK-M3-14 (the Benettin FTLE shadow) stops and is re-queued after M2's gate; M2's maths that seems to need the integrator is tested against exactly known trajectories through a trajectory interface the real integrator plugs into after M2, flagging only what can't be tested so; CI work outside the order, TASK-M2-38 and TASK-M2-56 finishing, and the gate checklist first are agreed *(no RQ)*"
- `decisions.md` § "R-923 — Integrator work before charts, for now: R-917's order (charts first, then integrator, then visuals) is set aside for now, on the owner's say; only integrator tasks that don't depend on the points the owner hasn't decided are taken, nothing that fixes how starting states reach the integrator on the GPU or the horizon as one value per dispatch; TASK-M3-14 stays stopped under R-918; no new chart task starts meanwhile *(amends R-917 in part, for now; no RQ)*"
- `docs/design/principia_chart_composition.md` § "18.4 Interactions"
- `docs/design/principia_chart_composition.md` § "18.5 Cost per column"
- `docs/design/principia_chart_composition.md` § "18.6 Tests"
- `docs/design/principia_chart_composition.md` § "18.7 The trajectory interface (R-918, R-919)"

## Deliverables
- The map's time coordinate and the playhead as the window's centre, in `crates/kernel` and `crates/engine`, read through TASK-M2-82's trajectory interface.
- A target time mapped to the latest boundary at or before it, in integers.

## Acceptance tests
- `cargo test -p engine time_coordinate` — G11 (`plan/MILESTONES.md`); a tilt toward time at 90° equals the time axis at its home extent (RQ-762's check); a target time maps to the latest boundary at or before it in integers. Measurement: composition § 18.5's C1 with per-sample stops, criterion as C1 wrote it: every row bitwise against a run with that row as horizon, `bounded` excepted (REQ-CHART-191).

## Notes
- Entered 11 Nov 2026 as N6 of the chart re-plan (`plan/charts/m2-chart-system-replan.md`) (applied per R-369: task scoping); its acceptance is the re-plan's § B.5 item 19. The plan documents are under `plan/charts/`; the lines above carry what they say.
- **Entered, not released (R-923).** R-923 (integrator work before charts, for now) holds the chart tasks TASK-M2-57 to TASK-M2-85, TASK-M2-82 excepted, and allows no new chart task meanwhile. Entering this task in `plan/tasks.yaml` does not lift the hold: it is a chart task, entered held, and it is not dispatched until the owner lifts R-923's hold.
- *13 Nov 2026 (R-987, the owner's message of 12 Nov 2026):* R-923 is lifted when the pull request porting the owner's answers to RQ-797 to RQ-806 merges; from then this task is dispatched as its dependencies allow, integrator (M3) tasks keeping priority for agent slots, unless it is tied to a question the owner hasn't answered, when it waits for that ruling.
- The † edge to TASK-M2-61 is kept (applied per R-369: task scoping): the playhead is the centre of that object's window `(c, Q)`, and the 90° check reads its per-coordinate scale.
- "With TASK-M2-83": TASK-M2-83 depends on this task.
