# TASK-M2-94 — The lock's solver core, from TASK-M2-73 (R-926 item 4)

- **Milestone:** M2
- **Closes:** REQ-CHART-193
- **Depends on:** TASK-M2-61
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** kernel; no split named

## Goal
The solver core of TASK-M2-73 (numerical inverses and the general solve), moved before TASK-M2-65 (paths under a lock) so the lock is one constraint re-solved (R-926 item 4). The re-solve follows its local minimiser continuously and stops where that ends (R-935, under RQ-759 (a)). It is also the nearest-point route's solver in TASK-M2-99 (placement).

## References
- `decisions.md` § "R-926 — The new chart system: a chart is a map from named coordinates to a system state and a time, composed with a flat window of a centre and a basis, every view edit an edit of the window and an axis change a tilt to 90°; time the last coordinate of every map, the playhead the window's centre along it; a morph a blend of two charts' states, linear in masses, momenta and time and a slerp on shape, exact at both ends, navigated by one pixel-space edit on both ends; the lock one constraint, "this pixel shows this system", re-solved; the CPU evaluates charts in f64 and the GPU only integrates records, with no chart code in the integrator; per-quad forward tables and no inverse on that path; a backwards table with a table search fallback for the fragment's needs only; no refusal path; three slope tiers; table debug views; a small quad's one-cell table the linearised decoder *(supersedes R-839; amends R-842; refines R-902)*"
- `decisions.md` § "R-935 — The chart re-plan's one-way questions, the plan's recommendation on each: exact or implicit slopes are the linearised decoder's `J_D`, central differences only the check (RQ-758); the lock's re-solve follows its local minimiser continuously, stops where that ends, and keeps the offered re-centre, the navigation stop and the stored-words select (RQ-759); a per-sample stop index on the integrator's grid with a new label "reached its time", the zoom along time floored at one boundary (RQ-761); the window's scale is R-507's and R-920's unit, time's the home time window (RQ-762); stage 1 measures each operation against its generic edit first (RQ-763); the GPU decode entries stay as test entries and R-837's cap is retired once measured (RQ-765); a large quad's forward table has one exact node per sample, R-90's trigger read on the narrowed f32 records (RQ-766); the table seeds a refinement on the chart's forward code (RQ-767); `ctx.chart.z` stays the forward value, carried in the table (RQ-768); stage 3 runs beside stages 1 and 2 (RQ-769); an in-plane pan or zoom still re-addresses the cache (RQ-770) *(closes RQ-758, RQ-759, RQ-761, RQ-762, RQ-763, RQ-765, RQ-766, RQ-767, RQ-768, RQ-769 and RQ-770; amends R-837, R-849, R-863 and R-864; refines R-90 and R-926)*"
- `decisions.md` § "R-947 — Placement, the re-plan addition's question (A): option (a), one placement function for "where is this state on this chart", two routes chosen by the chart, the existing reading where a chart has an inverse and the nearest point everywhere else, every blend included, refined to the lock's precision on the CPU *(no RQ; answers the re-plan addition's question (A); refines R-542 and R-926)*"
- `decisions.md` § "R-649 — Replication and verification targets are measurements, not gates: failing one never blocks building; each declares the tier it binds, (a) the GPU explorer, agreeing statistically or qualitatively only, or (b) the CPU IC inspector, where exact targets bind; three outcomes, PASS, EXPLAINED DIVERGENCE (shown by a measurement) and UNEXPLAINED (open, labelled, non-blocking); no default, physics, criterion, architecture or tolerance is changed and nothing is tuned per target in pursuit of a pass; a comparison is checked like-for-like before anything is called a failure; verification deepening comes after the core build *(amends R-594, R-636, R-637 and R-639 for external targets; applies to RQ-598's targets and every target in force)*"
- `decisions.md` § "R-923 — Integrator work before charts, for now: R-917's order (charts first, then integrator, then visuals) is set aside for now, on the owner's say; only integrator tasks that don't depend on the points the owner hasn't decided are taken, nothing that fixes how starting states reach the integrator on the GPU or the horizon as one value per dispatch; TASK-M3-14 stays stopped under R-918; no new chart task starts meanwhile *(amends R-917 in part, for now; no RQ)*"
- `docs/design/principia_chart_composition.md` § "17.8 Lock: a based homotopy"
- `docs/design/principia_chart_composition.md` § "10.6 Placement: one function for "where is this state on this chart""

## Deliverables
- The solver core: a least-squares solve over a map with a recorded branch rule, convergence reported per call, failures tagged.
- The measurement's test and report.

## Acceptance tests
- `cargo test -p engine lock_solver_core` — a least-squares solve over a map with a recorded branch rule, convergence reported per call, failures tagged. Measurement: the share of re-solves along a 64-frame morph that change basin under a four-seed restart against a continued solve; criterion: reported, and RQ-759 (a)'s continuity holds if the continued solve's largest step falls to at most 0.6 of the previous each time the frame step halves, twice (G8's criterion) (REQ-CHART-193).

## Notes
- Entered 11 Nov 2026 as N11 of the chart re-plan (`plan/charts/m2-chart-system-replan.md`) (applied per R-369: task scoping); its acceptance is the re-plan's § B.3 item 10. The plan documents are under `plan/charts/`; the lines above carry what they say.
- **Entered, not released (R-923).** R-923 (integrator work before charts, for now) holds the chart tasks TASK-M2-57 to TASK-M2-85, TASK-M2-82 excepted, and allows no new chart task meanwhile. Entering this task in `plan/tasks.yaml` does not lift the hold: it is a chart task, entered held, and it is not dispatched until the owner lifts R-923's hold.
- The † edge to TASK-M2-61 is kept (applied per R-369: task scoping): the solve is over that object's map, and its measurement runs along a morph of it.
- TASK-M2-73 gives this core up and keeps the rest; TASK-M2-65 depends on this task.
