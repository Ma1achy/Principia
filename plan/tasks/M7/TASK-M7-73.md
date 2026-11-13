# TASK-M7-73 — The five table debug views (R-925 item 6, R-926 item 10)

- **Milestone:** M7
- **Closes:** REQ-RENDER-124
- **Depends on:** TASK-M2-89, TASK-M2-92, TASK-M7-74
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** render; one view per commit

## Goal
Five debug views of the chart tables (R-925 item 6; R-926 item 10): the forward table by block, the forward-table check, the backwards table over its key, the route, and the distance off the chart.

## References
- `decisions.md` § "R-925 — Shader features work on every chart with no refusal, by the chart's closed-form shader code where it has some and otherwise by tables the CPU builds; Chart position stays, a table search its fallback where a chart has no backwards formula; the forward direction needs no table, the CPU evaluating the chart per quad and sending the result with the quad; "morph" means the system blend, a blend is a chart, and a window turn on one map is a tilt; a morph supports time running and time is a supported chart axis; debug views of the tables are required *(amends R-116 and R-850; refines R-542)*"
- `decisions.md` § "R-926 — The new chart system: a chart is a map from named coordinates to a system state and a time, composed with a flat window of a centre and a basis, every view edit an edit of the window and an axis change a tilt to 90°; time the last coordinate of every map, the playhead the window's centre along it; a morph a blend of two charts' states, linear in masses, momenta and time and a slerp on shape, exact at both ends, navigated by one pixel-space edit on both ends; the lock one constraint, "this pixel shows this system", re-solved; the CPU evaluates charts in f64 and the GPU only integrates records, with no chart code in the integrator; per-quad forward tables and no inverse on that path; a backwards table with a table search fallback for the fragment's needs only; no refusal path; three slope tiers; table debug views; a small quad's one-cell table the linearised decoder *(supersedes R-839; amends R-842; refines R-902)*"
- `decisions.md` § "R-917 — The build order: the chart system and its encoder and decoder, complete and tested, as M2; then the integrator; then everything visual; work in flight or approved finishes, and no new integrator or visual task starts until M2's gate passes, but fixes and what M2's own tests need; M2's gate, "fully working and tested", is the checklist G1 to G14 in `plan/MILESTONES.md`, the owner's lock-with-morphs test among it *(amends R-390 and R-627: the GUI track's exception, for new dispatch; refines R-914: the 217 tasks it released wait for the gate)*"
- `decisions.md` § "R-923 — Integrator work before charts, for now: R-917's order (charts first, then integrator, then visuals) is set aside for now, on the owner's say; only integrator tasks that don't depend on the points the owner hasn't decided are taken, nothing that fixes how starting states reach the integrator on the GPU or the horizon as one value per dispatch; TASK-M3-14 stays stopped under R-918; no new chart task starts meanwhile *(amends R-917 in part, for now; no RQ)*"
- `docs/design/principia_chart_composition.md` § "10.6 Placement: one function for "where is this state on this chart""

## Deliverables
- The five views in `crates/render`, each with its test.

## Acceptance tests
- `cargo test -p render table_debug_views` — for each of the five views (the forward table by block, the forward-table check, the backwards table over its key, the route, the distance off the chart), a test that renders a known table and reads back its colours, with a control (a table off by one node) that fails (REQ-RENDER-124).

## Notes
- Entered 11 Nov 2026 as N8 of the chart re-plan (`plan/charts/m2-chart-system-replan.md`) (applied per R-369: task scoping); its acceptance is the re-plan's § B.6 item 22, "Acceptance for each view". The plan documents are under `plan/charts/`; the lines above carry what they say.
- **Entered, not released (R-923).** R-923 (integrator work before charts, for now) holds the chart tasks TASK-M2-57 to TASK-M2-85, TASK-M2-82 excepted, and allows no new chart task meanwhile. Entering this task in `plan/tasks.yaml` does not lift the hold: it is a chart task, entered held, and it is not dispatched until the owner lifts R-923's hold, and it is visual work, which waits for M2's gate as well (R-917).
- *13 Nov 2026 (R-987, the owner's message of 12 Nov 2026):* R-923 is lifted when the pull request porting the owner's answers to RQ-797 to RQ-806 merges; from then this task is dispatched as its dependencies allow, integrator (M3) tasks keeping priority for agent slots, unless it is tied to a question the owner hasn't answered, when it waits for that ruling.
- The three † edges are kept (applied per R-369: task scoping): the views draw TASK-M2-89's forward table, TASK-M2-92's backwards table and its key, and the route and the distance off the chart of TASK-M7-74's fragment table route. The plans name no milestone for it beyond "after M2's gate": it is entered in M7 (applied per R-369: task scoping, listed for veto).
- Reviewers: its verify is a readback test and no screenshot, so `gui` is not named (the addition's § F.1: "gui if its verify is a screenshot").
