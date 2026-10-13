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
that RQ-273, which holds REQ-TOOL-011 for its uniform echo and Optional Fourier block rows, holds only this task.

## References
- `docs/contracts/principia_render_contract.md` § "Field views (one per field, every struct)"
- `docs/contracts/principia_render_contract.md` § "Part 6 — The debug catalogue (first build target)"
- `decisions.md` § "R-75 — The kernel keeps one debug mode *(closes RQ-26)*"
- `decisions.md` § "R-86 — The payload doc governs the eight payload items *(closes RQ-37)*"
- `decisions.md` § "R-111 — `SimResult` → `SimState`, `M` → `n_renorm`; the vocabulary lint covers the docs *(closes RQ-80)*"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
- `decisions.md` § "R-369 — Standing rule on autonomy: no size gate; decide and continue; ask the human only for the five kinds listed *(supersedes R-234 and R-367; amends R-175, R-204, R-208, R-211, R-264, R-283, R-290 and R-357)*"

## Deliverables
- The catalogue coverage checklist (a table in the PR, one row per render_contract Part 6 row; the DECODE, ensemble and
  live-effort rows marked with the requirement that holds them). Moved from TASK-M1-14.

## Acceptance tests
- Review checklist (qa): each row of render_contract Part 6's field-view table that a synthetic payload can show is ticked against the generated catalogue on a synthetic payload, the uniform echo reading `quality_tier`, `n_renorm` and thresholds; the DECODE row is REQ-RENDER-027's (M2), the ensemble rows REQ-TOOL-133's (M5) and the live effort heatmap REQ-TOOL-132's (M3) (REQ-TOOL-011).

## Notes
- Split from TASK-M1-14, applied per R-369 (TASK-M1-14's pre-flight). RQ-273 holds REQ-TOOL-011: the uniform echo row
  reads `SimUniforms` and `quality_tier` (TASK-M3-06's struct and TASK-M5-02's enum) and "thresholds *as currently
  bound*", which names none; and the Optional Fourier block row reads "the truncated-Fourier path when enabled", which
  nothing in the corpus or the plan builds or defines. This task waits for its ruling; TASK-M1-14 does not.
- RQ-80 ruled: R-111 — the uniform echo's `M` is `n_renorm` (render_contract conformed in step 7).
- RQ-94 ruled: R-113 — REQ-TOOL-011 ticks the rows a synthetic payload can show; the DECODE row is held by REQ-RENDER-027 (M2), the ensemble rows by REQ-TOOL-133 (M5, TASK-M5-18) and the live effort row by REQ-TOOL-132 (M3, TASK-M3-22).
- TASK-M2-25 depends on this task, since it needs REQ-TOOL-011 (its DECODE row is REQ-RENDER-027, R-113); TASK-M3-22
  and TASK-M7-17, which need REQ-TOOL-011 too, reach this task through TASK-M2-25.
