# TASK-M2-87 — The CPU-to-GPU record and table contract (R-926 items 5 and 6)

- **Milestone:** M2
- **Closes:** REQ-INT-115
- **Depends on:** TASK-M2-58
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** contract text, no code

## Goal
The record contract: the integrator's entry reads records the CPU wrote and never decodes (R-926 items 5 and 6). It lands with TASK-M2-59 and TASK-M2-60, which rewrite the lowering and scheduler contracts from R-926's dated notes.

## References
- `decisions.md` § "R-926 — The new chart system: a chart is a map from named coordinates to a system state and a time, composed with a flat window of a centre and a basis, every view edit an edit of the window and an axis change a tilt to 90°; time the last coordinate of every map, the playhead the window's centre along it; a morph a blend of two charts' states, linear in masses, momenta and time and a slerp on shape, exact at both ends, navigated by one pixel-space edit on both ends; the lock one constraint, "this pixel shows this system", re-solved; the CPU evaluates charts in f64 and the GPU only integrates records, with no chart code in the integrator; per-quad forward tables and no inverse on that path; a backwards table with a table search fallback for the fragment's needs only; no refusal path; three slope tiers; table debug views; a small quad's one-cell table the linearised decoder *(supersedes R-839; amends R-842; refines R-902)*"
- `decisions.md` § "R-935 — The chart re-plan's one-way questions, the plan's recommendation on each: exact or implicit slopes are the linearised decoder's `J_D`, central differences only the check (RQ-758); the lock's re-solve follows its local minimiser continuously, stops where that ends, and keeps the offered re-centre, the navigation stop and the stored-words select (RQ-759); a per-sample stop index on the integrator's grid with a new label "reached its time", the zoom along time floored at one boundary (RQ-761); the window's scale is R-507's and R-920's unit, time's the home time window (RQ-762); stage 1 measures each operation against its generic edit first (RQ-763); the GPU decode entries stay as test entries and R-837's cap is retired once measured (RQ-765); a large quad's forward table has one exact node per sample, R-90's trigger read on the narrowed f32 records (RQ-766); the table seeds a refinement on the chart's forward code (RQ-767); `ctx.chart.z` stays the forward value, carried in the table (RQ-768); stage 3 runs beside stages 1 and 2 (RQ-769); an in-plane pan or zoom still re-addresses the cache (RQ-770) *(closes RQ-758, RQ-759, RQ-761, RQ-762, RQ-763, RQ-765, RQ-766, RQ-767, RQ-768, RQ-769 and RQ-770; amends R-837, R-849, R-863 and R-864; refines R-90 and R-926)*"
- `decisions.md` § "R-923 — Integrator work before charts, for now: R-917's order (charts first, then integrator, then visuals) is set aside for now, on the owner's say; only integrator tasks that don't depend on the points the owner hasn't decided are taken, nothing that fixes how starting states reach the integrator on the GPU or the horizon as one value per dispatch; TASK-M3-14 stays stopped under R-918; no new chart task starts meanwhile *(amends R-917 in part, for now; no RQ)*"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 2 — The decoder"
- `docs/contracts/principia_lowering_contract.md` § "Part 3 — The baking rules (compile-time vs uniform)"
- `docs/contracts/principia_scheduler_contract.md` § "Principia — scheduler contract"

## Deliverables
- The record and table contract's text in the contracts, each replaced line kept as *Was*: the record is REQ-DEC-005's `(m, r, p)`, its descriptor and tag words, a branch word and the stop index of RQ-761 (R-935: a per-sample stop index); `SimState` a separate buffer (REQ-SCHED-012). The plan names no file beyond the lowering and scheduler contracts TASK-M2-59 and TASK-M2-60 rewrite: the PR says where the text went.

## Acceptance tests
- Review: the contracts state the record and table contract: the record's content (REQ-DEC-005's `(m, r, p)`, the descriptor and tag words, a branch word, the stop index of RQ-761), `SimState` a separate buffer (REQ-SCHED-012), each replaced line kept as *Was*; no measurement (REQ-INT-115).

## Notes
- Entered 11 Nov 2026 as N2 of the chart re-plan (`plan/charts/m2-chart-system-replan.md`) (applied per R-369: task scoping); its acceptance is the re-plan's § B.2 item 5. The plan documents are under `plan/charts/`; the lines above carry what they say.
- **Entered, not released (R-923).** R-923 (integrator work before charts, for now) holds the chart tasks TASK-M2-57 to TASK-M2-85, TASK-M2-82 excepted, and allows no new chart task meanwhile. Entering this task in `plan/tasks.yaml` does not lift the hold: it is a chart task, entered held, and it is not dispatched until the owner lifts R-923's hold.
- "With TASK-M2-59 and TASK-M2-60" is a place in the order, not an edge: the one edge the plan names is TASK-M2-58.
- TASK-M4-05 (flat-grid compute) reads these records and depends on this task.
- *11 Nov 2026 (R-926 item 5; the code review of PR #482, review 5852985982, F1):* this task carries the half of the re-plan's mark on REQ-DEC-007 (chart and links baked, the decode mode a per-quad flag; "baked for the CPU and the fragment, not the integrator") that no built task can: an integrator whose entry holds no chart code and reads records, as contract text, through REQ-INT-115. REQ-DEC-007 is reworded to baking wherever chart code is compiled, which TASK-M2-06 built, and points here for the contract. *11 Nov 2026 (the code re-check of PR #482, review 5853463481, N1):* this task is contract text with no code, so it cannot show that the built entry never decodes: that check is REQ-INT-116's, closed by TASK-M4-05 (flat-grid compute), which builds the entry and reads this task's records. *Was:* "an integrator whose entry holds no chart code and reads records, through REQ-INT-115. REQ-DEC-007 is reworded to baking wherever chart code is compiled, which TASK-M2-06 built, and points here for the rest."
