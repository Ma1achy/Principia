# TASK-M2-101 — Unfold in the CPU evaluator: past a fold the chart shows its second sheet (R-932, R-937)

- **Milestone:** M2
- **Closes:** REQ-CHART-204
- **Depends on:** TASK-M2-96, TASK-M2-66, TASK-M2-67
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** kernel; one kind

## Goal
The unfold kind of chart edge in the CPU evaluator (R-932), for every chart's edges that take it (R-938).

## References
- `decisions.md` § "R-932 — Chart edges: no blank areas; past every true edge the chart saturates, where there is a second sheet it unfolds onto it, and where the coordinate repeats it wraps; smearing and saturating are the same idea, and which mechanism an edge uses is physics review's call, saturating wherever the edge system itself is not a valid system; RQ-773 option (a): unfold, clamp and saturate become declared kinds per edge, and an unfold sheet is counted, a clamped pixel is not, a saturated pixel only where its system is not shown inside the square *(closes RQ-773; amends R-407, R-470, R-471, R-472, R-503, R-506, R-508, R-512, R-537, R-554, R-597 and R-708; refines R-408)*"
- `decisions.md` § "R-938 — RQ-772, which edges are built: every chart's edges now, not a first few; the time axis before zero shows the starting state now, the backward run later, reusing the one designed for the final-condition charts *(closes RQ-772)*"
- `decisions.md` § "R-941 — RQ-779, the default extension of each edge: per R-932; the mass simplex saturates, like the latent mass plane; the rest is confirmed by eye when the real charts render *(on RQ-779, which stays open)*"
- `decisions.md` § "R-407 — Past `[0,1]²` each chart axis extends by the type it declares: affine, periodic, pole-crossing or bounded, the default; a pixel that fails is hatched as forbidden *(closes RQ-262)*"
- `decisions.md` § "R-937 — RQ-771, the invariant charts' second sheet: option (a), the spin edges unfold onto the time-reversed sheet `vᵢ = vᵢ^(L) − a·wᵢ`, on the warped and the linear mapping, its meaning read from the seed R-934 rules; below the floor at a non-zero spin, where the lookup rule rejects, the drawn extension is the rigid rotation at the pixel's own spin, everywhere there, on the linear `(L_z, E)` and `(L_z, K)` charts and on Burrau `Φ_θ,L_z`, physics review agreeing with conditions C1 to C7; REQ-ENC-030's class rule binds lookups and the lock, never a drawn extension *(closes RQ-771; amends R-471 and R-508)*"
- `decisions.md` § "R-923 — Integrator work before charts, for now: R-917's order (charts first, then integrator, then visuals) is set aside for now, on the owner's say; only integrator tasks that don't depend on the points the owner hasn't decided are taken, nothing that fixes how starting states reach the integrator on the GPU or the horizon as one value per dispatch; TASK-M3-14 stays stopped under R-918; no new chart task starts meanwhile *(amends R-917 in part, for now; no RQ)*"
- `docs/design/principia_chart_reference.md` § "5.4 Past the unit square — each axis's extension type (R-407)"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 3 — Charts"

## Deliverables
- The per-edge declarations (each registry row and each shipped chart declares, per edge, the kinds allowed and its default) for the rows that take this kind; the kind applied before `Φ` in TASK-M2-67's evaluator, in R-407's order.

## Acceptance tests
- `cargo test -p engine edge_unfold` — unfold: samples past each edge that declares it, `s = 1 + δ` equal to sheet 2 at `1 − δ` bitwise; control: the unfold arm swapped for blank must fail; inside `[0,1]²` every chart bitwise unchanged (REQ-CHART-058's sweep still 0) (REQ-CHART-204).

## Notes
- Entered 11 Nov 2026 as N14a of the re-plan's addendum (`plan/charts/m2-chart-addendum-extensions-axes.md`) (applied per R-369: task scoping); its acceptance is the clause of the addendum's § C item 3 that names its kind, with that kind's control and "inside `[0,1]²` every chart bitwise unchanged". The plan documents are under `plan/charts/`; the lines above carry what they say.
- **Entered, not released (R-923).** R-923 (integrator work before charts, for now) holds the chart tasks TASK-M2-57 to TASK-M2-85, TASK-M2-82 excepted, and allows no new chart task meanwhile. Entering this task in `plan/tasks.yaml` does not lift the hold: it is a chart task, entered held, and it is not dispatched until the owner lifts R-923's hold.
- *13 Nov 2026 (R-987, the owner's message of 12 Nov 2026):* R-923 is lifted when the pull request porting the owner's answers to RQ-797 to RQ-806 merges; from then this task is dispatched as its dependencies allow, integrator (M3) tasks keeping priority for agent slots, unless it is tied to a question the owner hasn't answered, when it waits for that ruling.
- Split from N14 (the addendum § C item 3, per-edge extensions in the CPU evaluator and the count) by the kinds R-932 declares, every chart's edges being built now (R-938) (applied per R-369: task scoping). The per-edge declarations go with each kind's own task; the view state's override, blank's two controls and the debug view's per-pixel kind go with TASK-M2-105.
- A row whose default is still open under RQ-779 (each edge's default kind) is built to R-932's rule as R-941 says: the mass simplex saturates, and the owner confirms the rest by eye when the charts render. Which mechanism a row uses where the rule leaves a choice is physics review's call. Time before zero shows the starting state; the backward run comes later (R-938).
