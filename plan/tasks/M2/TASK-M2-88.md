# TASK-M2-88 — Slopes in three tiers: exact (dual numbers), implicit or none, a flag at each kink site (R-926 item 9)

- **Milestone:** M2
- **Closes:** REQ-DEC-058
- **Depends on:** TASK-M2-66, TASK-M2-67
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** kernel; splits by tier if over the limit

## Goal
Each map declares its slope tier (R-926 item 9): exact, by a dual-number type over the four bit-level traits; implicit; or none, a map with no exact slopes being still a valid chart. Exact or implicit slopes are the linearised decoder's `J_D`, central differences only the check (R-935, under RQ-758 (a)).

## References
- `decisions.md` § "R-926 — The new chart system: a chart is a map from named coordinates to a system state and a time, composed with a flat window of a centre and a basis, every view edit an edit of the window and an axis change a tilt to 90°; time the last coordinate of every map, the playhead the window's centre along it; a morph a blend of two charts' states, linear in masses, momenta and time and a slerp on shape, exact at both ends, navigated by one pixel-space edit on both ends; the lock one constraint, "this pixel shows this system", re-solved; the CPU evaluates charts in f64 and the GPU only integrates records, with no chart code in the integrator; per-quad forward tables and no inverse on that path; a backwards table with a table search fallback for the fragment's needs only; no refusal path; three slope tiers; table debug views; a small quad's one-cell table the linearised decoder *(supersedes R-839; amends R-842; refines R-902)*"
- `decisions.md` § "R-935 — The chart re-plan's one-way questions, the plan's recommendation on each: exact or implicit slopes are the linearised decoder's `J_D`, central differences only the check (RQ-758); the lock's re-solve follows its local minimiser continuously, stops where that ends, and keeps the offered re-centre, the navigation stop and the stored-words select (RQ-759); a per-sample stop index on the integrator's grid with a new label "reached its time", the zoom along time floored at one boundary (RQ-761); the window's scale is R-507's and R-920's unit, time's the home time window (RQ-762); stage 1 measures each operation against its generic edit first (RQ-763); the GPU decode entries stay as test entries and R-837's cap is retired once measured (RQ-765); a large quad's forward table has one exact node per sample, R-90's trigger read on the narrowed f32 records (RQ-766); the table seeds a refinement on the chart's forward code (RQ-767); `ctx.chart.z` stays the forward value, carried in the table (RQ-768); stage 3 runs beside stages 1 and 2 (RQ-769); an in-plane pan or zoom still re-addresses the cache (RQ-770) *(closes RQ-758, RQ-759, RQ-761, RQ-762, RQ-763, RQ-765, RQ-766, RQ-767, RQ-768, RQ-769 and RQ-770; amends R-837, R-849, R-863 and R-864; refines R-90 and R-926)*"
- `decisions.md` § "R-633 — Every central-difference Jacobian of the decode takes the encode contract's step; the steer's `J⁺` drops each singular value below `c·σ₁`, `c` a calibration; a chart that holds a block by a physical construction has that construction's physical parameters as hidden coordinates, tilted and sliced by value, and TASK-M2-36 splits; an unusable copy is a bit test on `state` and `budget_word` *(closes RQ-600, RQ-601, RQ-603 and RQ-604; amends R-473; refines R-550)*"
- `decisions.md` § "R-923 — Integrator work before charts, for now: R-917's order (charts first, then integrator, then visuals) is set aside for now, on the owner's say; only integrator tasks that don't depend on the points the owner hasn't decided are taken, nothing that fixes how starting states reach the integrator on the GPU or the horizon as one value per dispatch; TASK-M3-14 stays stopped under R-918; no new chart task starts meanwhile *(amends R-917 in part, for now; no RQ)*"
- `docs/design/principia_chart_composition.md` § "3. The primitive registry"

## Deliverables
- The dual-number type over the four bit-level traits, and each map's declared tier in its registry row (TASK-M2-66's rows).
- The flag a tier-1 or tier-2 map sets at each kink site.
- Measurement M-B's test and its report.

## Acceptance tests
- `cargo test -p kernel slope_tiers` — each map declares its tier; a tier-1 or tier-2 map sets the flag at each listed kink site; a tier-3 map is accepted with no slope, each sample evaluated on the CPU. Measurement M-B, for each tier-1 or tier-2 map: the dual slope against a central difference at R-633's step, `h_k = ε^{1/3}·max(1, |x_k|)` in f64 (the step REQ-DEC-034 names), at points away from the flagged sites by more than that step; criterion, written before the run: per column, agreement within the central difference's error at that step, its truncation `h_k²·|f'''|/6` plus its rounding `ε·|f|/h_k`, each of order `ε^{2/3}`, with `|f'''|` bounded from the map's own closed form and stated before the run; the control flips the sign of a column whose norm is at least 100 times that bound, and must fail (REQ-DEC-058).

## Notes
- Entered 11 Nov 2026 as N3 of the chart re-plan (`plan/charts/m2-chart-system-replan.md`) (applied per R-369: task scoping); its acceptance is the re-plan's § B.4 item 13, with measurement M-B. The plan documents are under `plan/charts/`; the lines above carry what they say.
- **Entered, not released (R-923).** R-923 (integrator work before charts, for now) holds the chart tasks TASK-M2-57 to TASK-M2-85, TASK-M2-82 excepted, and allows no new chart task meanwhile. Entering this task in `plan/tasks.yaml` does not lift the hold: it is a chart task, entered held, and it is not dispatched until the owner lifts R-923's hold.
- Both † edges are kept (applied per R-369: task scoping): the tier is declared in TASK-M2-66's registry rows, and a tier-3 map's samples are TASK-M2-67's CPU evaluator's.
- REQ-DEC-034 and REQ-DEC-036 (`J_D` by central differences) are brought into line with R-935 and stay with their closing tasks, TASK-M6-07 and TASK-M5-04, which read this task's slopes.
- The re-plan's open item O2 (exact slopes not built in the prototype) is closed by M-B.
