# TASK-M1-19 — The render_contract Part 6 catalogue checklist (REQ-TOOL-011)

- **Milestone:** M1
- **Closes:** REQ-TOOL-011
- **Depends on:** TASK-M1-14
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~60 lines, mostly the PR's checklist table

## Goal
The catalogue is ticked against render_contract Part 6's field-view table, for the rows a synthetic payload can show
(R-113): each row is checked against the generated catalogue on a synthetic payload, and the rows R-113 split off are
marked with the requirement that holds them. Split from TASK-M1-14 (applied per R-369, TASK-M1-14's pre-flight) so
that RQ-273, which held REQ-TOOL-011 for its uniform echo and Optional Fourier block rows, held only this task. R-420
ruled it: the echo row is REQ-TOOL-164's (TASK-M5-31) and the Fourier row is removed from Part 6, so this task waits
on nothing.
RQ-275 and RQ-276, from this task's pre-flight, held two cells of the checklist: the quad fields row's
payload/status flags and the structural overlays row's visible-set and locked/stale overlays (Notes). R-424 and R-425
ruled them: contains-ensemble, contains-FTLE and the schema version are checked here against `has_ensemble()`,
`has_ftle` and `PAYLOAD_SCHEMA_VERSION`; sim-failed, cache-valid, visible-set and locked/stale are marked with the
M5 requirements that hold them; this task waits on nothing.

## References
- `docs/contracts/principia_render_contract.md` § "Field views (one per field, every struct)"
- `docs/contracts/principia_render_contract.md` § "Part 6 — The debug catalogue (first build target)"
- `decisions.md` § "R-75 — The kernel keeps one debug mode *(closes RQ-26)*"
- `decisions.md` § "R-86 — The payload doc governs the eight payload items *(closes RQ-37)*"
- `decisions.md` § "R-111 — `SimResult` → `SimState`, `M` → `n_renorm`; the vocabulary lint covers the docs *(closes RQ-80)*"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
- `decisions.md` § "R-420 — The uniform echo row moves to a task after TASK-M5-02 and TASK-M3-06, which names its thresholds; the Optional Fourier block row is removed from render_contract Part 6 and parked as a possible future feature *(closes RQ-273)*"
- `decisions.md` § "R-424 — The quad fields row's flags: contains-ensemble, contains-FTLE and the schema version are checked at M1 against `has_ensemble()`, `has_ftle` and `PAYLOAD_SCHEMA_VERSION`; sim-failed moves to the `QuadReduction` task and cache-valid to the M5 cache task *(closes RQ-275)*"
- `decisions.md` § "R-425 — The visible-set and locked/stale overlays move to TASK-M5-29 under a new defining requirement, which reuses the corpus's existing meanings (`quad_state` 4 for "stale") rather than defining new ones *(closes RQ-276)*"
- `docs/contracts/principia_render_contract.md` § "Part 2 — Fixed pipeline, swappable slots"
- `decisions.md` § "R-369 — Standing rule on autonomy: no size gate; decide and continue; ask the human only for the five kinds listed *(supersedes R-234 and R-367; amends R-175, R-204, R-208, R-211, R-264, R-283, R-290 and R-357)*"

## Deliverables
- The catalogue coverage checklist (a table in the PR, one row per render_contract Part 6 row; the DECODE, ensemble,
  live-effort and uniform echo rows marked with the requirement that holds them). Moved from TASK-M1-14. In the quad
  fields row, the contains-ensemble, contains-FTLE and schema version cells are ticked against the prelude's
  `has_ensemble()` and `has_ftle` and the generated `PAYLOAD_SCHEMA_VERSION`, and the sim-failed and cache-valid cells
  marked with REQ-PAY-096 and REQ-SCHED-099; in the structural overlays row, the visible-set and locked/stale cells
  marked with REQ-TOOL-166 (R-424, R-425).

## Acceptance tests
- Review checklist (qa): each row of render_contract Part 6's field-view table that a synthetic payload can show is ticked against the generated catalogue on a synthetic payload; the DECODE row is REQ-RENDER-027's (M2), the ensemble rows REQ-TOOL-133's (M5), the live effort heatmap REQ-TOOL-132's (M3) and the uniform echo row REQ-TOOL-164's (M5, R-420); in the quad fields row, contains-ensemble is ticked against the prelude's `has_ensemble()` with E toggled (the spread view's gate), contains-FTLE against `has_ftle` in both tier variants, and the schema version against the generated `PAYLOAD_SCHEMA_VERSION`, each on a synthetic payload with no `RenderQuad` or `CtxQuad` member added, while sim-failed is REQ-PAY-096's (M5, TASK-M5-17) and cache-valid REQ-SCHED-099's (M5, TASK-M5-08) (R-424); in the structural overlays row, visible-set and locked/stale are REQ-TOOL-166's (M5, TASK-M5-29, R-425); the table has no Optional Fourier block row (removed by R-420) (REQ-TOOL-011).

## Notes
- Split from TASK-M1-14, applied per R-369 (TASK-M1-14's pre-flight). RQ-273 held REQ-TOOL-011: the uniform echo row
  read `SimUniforms` and `quality_tier` (TASK-M3-06's struct and TASK-M5-02's enum) and "thresholds *as currently
  bound*", which named none; and the Optional Fourier block row read "the truncated-Fourier path when enabled", which
  nothing in the corpus or the plan built or defined. R-420 (11 Oct 2026) rules it: the echo row moves to TASK-M5-31
  (REQ-TOOL-164, REQ-TOOL-165) and the Fourier row is removed from Part 6 and parked (philosophy §7.6a); this task
  waits on nothing.
- RQ-80 ruled: R-111 — the uniform echo's `M` is `n_renorm` (render_contract conformed in step 7).
- RQ-94 ruled: R-113 — REQ-TOOL-011 ticks the rows a synthetic payload can show; the DECODE row is held by REQ-RENDER-027 (M2), the ensemble rows by REQ-TOOL-133 (M5, TASK-M5-18) and the live effort row by REQ-TOOL-132 (M3, TASK-M3-22).
- RQ-275 and RQ-276 (this task's read-only pre-flight; REQ-TOOL-011 carried `rq: RQ-275` and `rq: RQ-276`): the quad
  fields row's payload/status flags (sim-failed, cache-valid, contains-ensemble, contains-FTLE, schema version) were
  read from `ctx.quad`, but neither `RenderQuad` (dd_generation_root §3.7a) nor the stain's `CtxQuad` carries them and
  no contract defined them (RQ-275); the structural overlays row's visible-set and locked/stale overlays were defined
  and built nowhere (RQ-276). R-424 (13 Oct 2026) rules RQ-275: contains-ensemble and contains-FTLE are checked
  against `has_ensemble()` and `has_ftle` (as RQ-239 did), the schema version against `PAYLOAD_SCHEMA_VERSION` for
  M1; sim-failed moves to TASK-M5-17 (REQ-PAY-096) and cache-valid to TASK-M5-08 (REQ-SCHED-099). R-425 (13 Oct
  2026) rules RQ-276, option (b): both overlays move to TASK-M5-29 (REQ-TOOL-166). This task waits on nothing.
- TASK-M2-25 depends on this task, since it needs REQ-TOOL-011 (its DECODE row is REQ-RENDER-027, R-113); TASK-M3-22
  and TASK-M7-17, which need REQ-TOOL-011 too, reach this task through TASK-M2-25.
