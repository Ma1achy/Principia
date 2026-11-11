# TASK-M2-103 — Saturate in the CPU evaluator (R-932, R-941)

- **Milestone:** M2
- **Closes:** REQ-CHART-206
- **Depends on:** TASK-M2-95, TASK-M2-66, TASK-M2-67
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** kernel; one kind

## Goal
The saturate kind of chart edge in the CPU evaluator (R-932), for every chart's edges that take it (R-938).

## References
- `decisions.md` § "R-932 — Chart edges: no blank areas; past every true edge the chart saturates, where there is a second sheet it unfolds onto it, and where the coordinate repeats it wraps; smearing and saturating are the same idea, and which mechanism an edge uses is physics review's call, saturating wherever the edge system itself is not a valid system; RQ-773 option (a): unfold, clamp and saturate become declared kinds per edge, and an unfold sheet is counted, a clamped pixel is not, a saturated pixel only where its system is not shown inside the square *(closes RQ-773; amends R-407, R-470, R-471, R-472, R-503, R-506, R-508, R-512, R-537, R-554, R-597 and R-708; refines R-408)*"
- `decisions.md` § "R-938 — RQ-772, which edges are built: every chart's edges now, not a first few; the time axis before zero shows the starting state now, the backward run later, reusing the one designed for the final-condition charts *(closes RQ-772)*"
- `decisions.md` § "R-941 — RQ-779, the default extension of each edge: per R-932; the mass simplex saturates, like the latent mass plane; the rest is confirmed by eye when the real charts render *(on RQ-779, which stays open)*"
- `decisions.md` § "R-407 — Past `[0,1]²` each chart axis extends by the type it declares: affine, periodic, pole-crossing or bounded, the default; a pixel that fails is hatched as forbidden *(closes RQ-262)*"
- `decisions.md` § "R-923 — Integrator work before charts, for now: R-917's order (charts first, then integrator, then visuals) is set aside for now, on the owner's say; only integrator tasks that don't depend on the points the owner hasn't decided are taken, nothing that fixes how starting states reach the integrator on the GPU or the horizon as one value per dispatch; TASK-M3-14 stays stopped under R-918; no new chart task starts meanwhile *(amends R-917 in part, for now; no RQ)*"
- `docs/design/principia_chart_reference.md` § "5.4 Past the unit square — each axis's extension type (R-407)"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 3 — Charts"

## Deliverables
- The per-edge declarations (each registry row and each shipped chart declares, per edge, the kinds allowed and its default) for the rows that take this kind; the kind applied before `Φ` in TASK-M2-67's evaluator, in R-407's order.

## Acceptance tests
- `cargo test -p engine edge_saturate` — saturate: samples past each edge that declares it are monotone and inside its declared band; the mass simplex saturates (R-941); under the edge-reaching simplex link a pixel with `M₀₁ < 10⁻⁵` (R-597, REQ-CHART-087) saturates; control: the saturate arm swapped for blank must fail; inside `[0,1]²` every chart bitwise unchanged (REQ-CHART-058's sweep still 0) (REQ-CHART-206).

## Notes
- Entered 11 Nov 2026 as N14c of the re-plan's addendum (`plan/charts/m2-chart-addendum-extensions-axes.md`) (applied per R-369: task scoping); its acceptance is the clause of the addendum's § C item 3 that names its kind, with that kind's control and "inside `[0,1]²` every chart bitwise unchanged". The plan documents are under `plan/charts/`; the lines above carry what they say.
- **Entered, not released (R-923).** R-923 (integrator work before charts, for now) holds the chart tasks TASK-M2-57 to TASK-M2-85, TASK-M2-82 excepted, and allows no new chart task meanwhile. Entering this task in `plan/tasks.yaml` does not lift the hold: it is a chart task, entered held, and it is not dispatched until the owner lifts R-923's hold.
- Split from N14 (the addendum § C item 3, per-edge extensions in the CPU evaluator and the count) by the kinds R-932 declares, every chart's edges being built now (R-938) (applied per R-369: task scoping). The per-edge declarations go with each kind's own task; the view state's override, blank's two controls and the debug view's per-pixel kind go with TASK-M2-105.
- A row whose default is still open under RQ-779 (each edge's default kind) is built to R-932's rule as R-941 says: the mass simplex saturates, and the owner confirms the rest by eye when the charts render. Which mechanism a row uses where the rule leaves a choice is physics review's call. Time before zero shows the starting state; the backward run comes later (R-938).
- *11 Nov 2026 (the physics review of PR #482, review 5852979675, P3):* gains TASK-M2-95 (the extension kinds in the contracts) in Depends on. This task builds the per-edge declarations for the rows that take its kind, and which edges take a kind, its band and its place in R-408's order are TASK-M2-95's per-edge table, physics-reviewed under R-407's clause 3 (REQ-CHART-194); the addendum orders N12, then N13, then N14. Unfold and clamp reach TASK-M2-95 through TASK-M2-96; this kind did not. *Was (Depends on):* "TASK-M2-66, TASK-M2-67".
