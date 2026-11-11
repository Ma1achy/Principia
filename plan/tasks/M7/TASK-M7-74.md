# TASK-M7-74 — The fragment table route: the GPU side of the backwards table, for Chart position and the presets (R-925 items 1 and 2)

- **Milestone:** M7
- **Closes:** REQ-RENDER-125, REQ-PERF-122
- **Depends on:** TASK-M2-92, TASK-M2-99
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** none
- **Size:** render; no split named

## Goal
The fragment's table route: where a chart has no closed-form shader code, Chart position and the decode and round-trip presets read TASK-M2-92's backwards table on the GPU, so shader features work on every chart with no refusal (R-925 items 1 and 2). R-926 item 7's "Approximate is acceptable during a morph" binds this route only (R-947), and it is measured against `place` (TASK-M2-99) on the CPU.

## References
- `decisions.md` § "R-925 — Shader features work on every chart with no refusal, by the chart's closed-form shader code where it has some and otherwise by tables the CPU builds; Chart position stays, a table search its fallback where a chart has no backwards formula; the forward direction needs no table, the CPU evaluating the chart per quad and sending the result with the quad; "morph" means the system blend, a blend is a chart, and a window turn on one map is a tilt; a morph supports time running and time is a supported chart axis; debug views of the tables are required *(amends R-116 and R-850; refines R-542)*"
- `decisions.md` § "R-926 — The new chart system: a chart is a map from named coordinates to a system state and a time, composed with a flat window of a centre and a basis, every view edit an edit of the window and an axis change a tilt to 90°; time the last coordinate of every map, the playhead the window's centre along it; a morph a blend of two charts' states, linear in masses, momenta and time and a slerp on shape, exact at both ends, navigated by one pixel-space edit on both ends; the lock one constraint, "this pixel shows this system", re-solved; the CPU evaluates charts in f64 and the GPU only integrates records, with no chart code in the integrator; per-quad forward tables and no inverse on that path; a backwards table with a table search fallback for the fragment's needs only; no refusal path; three slope tiers; table debug views; a small quad's one-cell table the linearised decoder *(supersedes R-839; amends R-842; refines R-902)*"
- `decisions.md` § "R-947 — Placement, the re-plan addition's question (A): option (a), one placement function for "where is this state on this chart", two routes chosen by the chart, the existing reading where a chart has an inverse and the nearest point everywhere else, every blend included, refined to the lock's precision on the CPU *(no RQ; answers the re-plan addition's question (A); refines R-542 and R-926)*"
- `decisions.md` § "R-945 — R-926 item 7's "ONLY" list was not exhaustive: the backwards direction serves anything that places a running or saved state on a chart, traces and marks on the CPU among them *(no RQ; refines R-926)*"
- `decisions.md` § "R-186 — GitHub-hosted runners first; no self-hosted runner *(amends R-110, R-169, R-174)*"
- `decisions.md` § "R-917 — The build order: the chart system and its encoder and decoder, complete and tested, as M2; then the integrator; then everything visual; work in flight or approved finishes, and no new integrator or visual task starts until M2's gate passes, but fixes and what M2's own tests need; M2's gate, "fully working and tested", is the checklist G1 to G14 in `plan/MILESTONES.md`, the owner's lock-with-morphs test among it *(amends R-390 and R-627: the GUI track's exception, for new dispatch; refines R-914: the 217 tasks it released wait for the gate)*"
- `decisions.md` § "R-923 — Integrator work before charts, for now: R-917's order (charts first, then integrator, then visuals) is set aside for now, on the owner's say; only integrator tasks that don't depend on the points the owner hasn't decided are taken, nothing that fixes how starting states reach the integrator on the GPU or the horizon as one value per dispatch; TASK-M3-14 stays stopped under R-918; no new chart task starts meanwhile *(amends R-917 in part, for now; no RQ)*"
- `docs/design/principia_chart_composition.md` § "10.6 Placement: one function for "where is this state on this chart""

## Deliverables
- The backwards pass in `crates/render`: TASK-M2-92's table uploaded and searched in the fragment, for Chart position and the presets.
- The route's difference from `place` on the CPU, measured and reported.
- The benchmark of the backwards pass on Metal, and its report.

## Acceptance tests
- `cargo test -p render fragment_table_route` — on a chart with no closed-form shader code, Chart position and the presets read the backwards table in the fragment and nothing is refused; the route's answer is measured against `place` on the CPU (REQ-CHART-199) and the difference reported. The sources give no bound for that difference (R-926 item 7: approximate is acceptable during a morph, which binds this route only): none is asserted and none is chosen here (REQ-RENDER-125).
- `cargo xtask bench backwards_pass` — the per-sample cost of the backwards pass on Metal at 512 × 512, reported against the 16.7 ms frame budget; PASS if the pass takes at most a quarter of it; benchmark: waits for the human's Mac (R-186) (REQ-PERF-122).

## Notes
- Entered 11 Nov 2026 as N9 of the chart re-plan (`plan/charts/m2-chart-system-replan.md`) (applied per R-369: task scoping); its acceptance is the re-plan's § B.6 item 22, "Measurement"; the addition's § A.3, "Refined to the lock's precision". The plan documents are under `plan/charts/`; the lines above carry what they say.
- **Entered, not released (R-923).** R-923 (integrator work before charts, for now) holds the chart tasks TASK-M2-57 to TASK-M2-85, TASK-M2-82 excepted, and allows no new chart task meanwhile. Entering this task in `plan/tasks.yaml` does not lift the hold: it is a chart task, entered held, and it is not dispatched until the owner lifts R-923's hold, and it is visual work, which waits for M2's gate as well (R-917).
- Both edges are the plans' own: TASK-M2-92 (this is its GPU side) and TASK-M2-99 (it is measured against `place`). The plans name no milestone for it beyond "after M2's gate": it is entered in M7 (applied per R-369: task scoping, listed for veto).
