# TASK-M2-95 — The extension kinds in the contracts: the kinds per edge, each shipped chart's per-edge table, the second sheet and the clamp's metric defined (R-932, R-937, R-938)

- **Milestone:** M2
- **Closes:** REQ-CHART-194, REQ-CHART-195
- **Depends on:** TASK-M2-58
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** contract text and requirements, no code

## Goal
The contracts state the extension kinds per chart edge (R-932: no blank areas; past every true edge the chart saturates, unfolds onto a second sheet where there is one, and wraps where the coordinate repeats), for every chart's edges (R-938), with R-937's definitions: the sheet, and the clamp below the floor. It lands with TASK-M2-59 and TASK-M2-60, after TASK-M2-58 has carried the answers to RQ-771 to RQ-781.

## References
- `decisions.md` § "R-932 — Chart edges: no blank areas; past every true edge the chart saturates, where there is a second sheet it unfolds onto it, and where the coordinate repeats it wraps; smearing and saturating are the same idea, and which mechanism an edge uses is physics review's call, saturating wherever the edge system itself is not a valid system; RQ-773 option (a): unfold, clamp and saturate become declared kinds per edge, and an unfold sheet is counted, a clamped pixel is not, a saturated pixel only where its system is not shown inside the square *(closes RQ-773; amends R-407, R-470, R-471, R-472, R-503, R-506, R-508, R-512, R-537, R-554, R-597 and R-708; refines R-408)*"
- `decisions.md` § "R-937 — RQ-771, the invariant charts' second sheet: option (a), the spin edges unfold onto the time-reversed sheet `vᵢ = vᵢ^(L) − a·wᵢ`, on the warped and the linear mapping, its meaning read from the seed R-934 rules; below the floor at a non-zero spin, where the lookup rule rejects, the drawn extension is the rigid rotation at the pixel's own spin, everywhere there, on the linear `(L_z, E)` and `(L_z, K)` charts and on Burrau `Φ_θ,L_z`, physics review agreeing with conditions C1 to C7; REQ-ENC-030's class rule binds lookups and the lock, never a drawn extension *(closes RQ-771; amends R-471 and R-508)*"
- `decisions.md` § "R-938 — RQ-772, which edges are built: every chart's edges now, not a first few; the time axis before zero shows the starting state now, the backward run later, reusing the one designed for the final-condition charts *(closes RQ-772)*"
- `decisions.md` § "R-941 — RQ-779, the default extension of each edge: per R-932; the mass simplex saturates, like the latent mass plane; the rest is confirmed by eye when the real charts render *(on RQ-779, which stays open)*"
- `decisions.md` § "R-942 — The chart addendum's remaining questions, the plan's recommendation on each: the per-edge choice lives in the registry row and, as an override, in the view state, a re-addressing kind off the sim key and a kind that shows new systems on it (RQ-775); the backwards table holds sheet 1 only, a state looked up as itself and as its time reverse (RQ-776); no small term is dropped from an axis's name (RQ-778); the computed axis is the primary scale on every chart and every frame, the no-ticks rule mid-morph withdrawn (RQ-780); the view's centre may cross onto an unfold sheet and stops at a clamped or saturated area (RQ-781); RQ-778 and RQ-780 read with the dual ruler *(closes RQ-775, RQ-776, RQ-778, RQ-780 and RQ-781; amends R-472; refines R-469, R-476 and R-926)*"
- `decisions.md` § "R-407 — Past `[0,1]²` each chart axis extends by the type it declares: affine, periodic, pole-crossing or bounded, the default; a pixel that fails is hatched as forbidden *(closes RQ-262)*"
- `decisions.md` § "R-923 — Integrator work before charts, for now: R-917's order (charts first, then integrator, then visuals) is set aside for now, on the owner's say; only integrator tasks that don't depend on the points the owner hasn't decided are taken, nothing that fixes how starting states reach the integrator on the GPU or the horizon as one value per dispatch; TASK-M3-14 stays stopped under R-918; no new chart task starts meanwhile *(amends R-917 in part, for now; no RQ)*"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 3 — Charts"
- `docs/design/principia_chart_reference.md` § "5.4 Past the unit square — each axis's extension type (R-407)"
- `docs/design/principia_chart_reference.md` § "2.2 Deterministic momentum construction"

## Deliverables
- `principia_chart_decoder_contract.md` Part 3 and `principia_chart_reference.md` § 5.4 rewritten from the answers, each replaced line kept as *Was*: the kinds (affine, periodic, pole-crossing, unfold, clamp, saturate, blank, which is R-407's "bounded" and has the one name "blank", and, where a ruling declares one, a reflection or a new repeat map, each a new kind under R-407's clause 3), declared per edge, each with its primary range and its place in R-408's classification order.
- Each shipped chart's per-edge table, every chart's edges (R-938), physics-reviewed under R-407's clause 3; the seam along `E = U` declared in it.
- The definitions of REQ-CHART-195, in chart_reference § 2.2.
- R-471's E2 amended (the linear option's infeasible pixels take their edge's declared kind); under the edge-reaching simplex link a pixel with `M₀₁ < 10⁻⁵` (R-597, REQ-CHART-087, REQ-GUI-357) saturates, this task writing the definition. A pixel that energy normalisation cannot reach (R-598) stays not a system: R-955 (11 Nov 2026) answers RQ-784 with option (a), R-598 standing. *Was:* "stays not a system until RQ-784 is answered."

## Acceptance tests
- Review: `principia_chart_decoder_contract.md` Part 3 and `principia_chart_reference.md` § 5.4 are rewritten from the answers, each replaced line kept as *Was*: the kinds, declared per edge, each with its primary range and its place in R-408's classification order; each shipped chart's per-edge table, physics-reviewed under R-407's clause 3; REQ-ENC-030's metric named as the clamp's where the clamp projects; REQ-CHART-054 and REQ-CHART-056 in line with it; no measurement (REQ-CHART-194).
- Definition: chart_reference § 2.2 defines the second sheet (R-937: the spin edges unfold onto the time-reversed sheet, its meaning read from the seed R-934 rules) and the clamp below the floor with its metric `|ΔE|` at fixed `L_z`; the physics reviewer approved (REQ-CHART-195).

## Notes
- Entered 11 Nov 2026 as N12 of the re-plan's addendum (`plan/charts/m2-chart-addendum-extensions-axes.md`) (applied per R-369: task scoping); its acceptance is the addendum's § C item 1, and its answers section's N12 bullet (R-932, R-937, R-938). The plan documents are under `plan/charts/`; the lines above carry what they say.
- **Entered, not released (R-923).** R-923 (integrator work before charts, for now) holds the chart tasks TASK-M2-57 to TASK-M2-85, TASK-M2-82 excepted, and allows no new chart task meanwhile. Entering this task in `plan/tasks.yaml` does not lift the hold: it is a chart task, entered held, and it is not dispatched until the owner lifts R-923's hold.
- *13 Nov 2026 (R-987, the owner's message of 12 Nov 2026):* R-923 is lifted when the pull request porting the owner's answers to RQ-797 to RQ-806 merges; from then this task is dispatched as its dependencies allow, integrator (M3) tasks keeping priority for agent slots, unless it is tied to a question the owner hasn't answered, when it waits for that ruling.
- "With TASK-M2-59 and TASK-M2-60" is a place in the order, not an edge.
- TASK-M2-95 to TASK-M2-102 must meet the physics reviewer's conditions C1 to C7 on the clamp below the floor (R-937 records them in full).
- RQ-779 (each edge's default kind) stays open: rows whose default is open are written to R-932's rule as R-941 says, the mass simplex saturating, the owner confirming the rest by eye when the charts render.
- REQ-CHART-054 and REQ-CHART-056 are brought into line with R-932 and stay with TASK-M8-87, their closing task.
