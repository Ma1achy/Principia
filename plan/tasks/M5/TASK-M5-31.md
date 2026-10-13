# TASK-M5-31 — The uniform echo view (R-420)

- **Milestone:** M5
- **Closes:** REQ-TOOL-164, REQ-TOOL-165
- **Depends on:** TASK-M5-02, TASK-M3-06
- **Needs (earlier milestones):** REQ-INT-026, REQ-TOOL-011
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~150 lines

## Goal
**R-420 (RQ-273, option (a)):** render_contract Part 6's uniform echo row, moved off the M1 checklist, is built here,
after TASK-M5-02 (`quality_tier`) and TASK-M3-06 (`SimUniforms`): flat swatches of `quality_tier`, `n_renorm` (R-111)
and the thresholds as currently bound, so that a slider moved but not rebound shows. This task names the thresholds
the echo shows, and the binding it reads `quality_tier` from, since TASK-M3-06's `SimUniforms` holds no tier (a
definition, R-72).

## References
- `decisions.md` § "R-420 — The uniform echo row moves to a task after TASK-M5-02 and TASK-M3-06, which names its thresholds; the Optional Fourier block row is removed from render_contract Part 6 and parked as a possible future feature *(closes RQ-273)*"
- `docs/contracts/principia_render_contract.md` § "Field views (one per field, every struct)"
- `docs/contracts/principia_render_contract.md` § "Part 6 — The debug catalogue (first build target)"
- `decisions.md` § "R-111 — `SimResult` → `SimState`, `M` → `n_renorm`; the vocabulary lint covers the docs *(closes RQ-80)*"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/contracts/principia_integrator_contract.md` § "Part 3 — Parameter ownership (who owns what)"
- `docs/design/principia_memory_tiers.md` § "4. The six quality tiers"

## Deliverables
- `docs/contracts/principia_render_contract.md` Part 6: the thresholds the echo shows, and where it reads
  `quality_tier`, written into the uniform echo row's paragraph with the commit's "Removed lines" note (REQ-TOOL-165).
- `crates/render`: the uniform echo view, reading the bound uniforms (not a CPU-side copy), one flat swatch per echoed
  value.
- Tests: `uniform_echo_rebinds`, with its negative control.

## Acceptance tests
- `cargo test -p render uniform_echo_rebinds` — changing each echoed value (`quality_tier`, `n_renorm`, each named threshold) and rebinding changes its swatch; control: a change that is not rebound leaves the swatch as it was, and the check fails (REQ-TOOL-164).
- Review checklist (qa): the uniform echo row of render_contract Part 6 is ticked against the generated catalogue on a real binding (REQ-TOOL-164).
- Review checklist (physics): render_contract Part 6 names each threshold the echo shows and where `quality_tier` is read from, and the echo reads exactly those; the doc change is in this PR and the physics reviewer approves it before merge (REQ-TOOL-165).

## Notes
- New task, applied per R-369 in R-420's port: the echo row needs TASK-M5-02's tier and TASK-M3-06's uniforms, and
  TASK-M1-19 cannot depend on TASK-M5-02 without a cycle (TASK-M5-02 is downstream of TASK-M2-25, which depends on
  TASK-M1-19). It is not a line on TASK-M5-12, whose knobs and blast radii are a different deliverable.
- `SimUniforms`' threshold candidates (TASK-M3-06): `r_sub`, `r_coll`, `tau`, `escape_window`, `r_close`, `eps_e`,
  `eps_l`. Which the echo shows is this task's definition (REQ-TOOL-165), not a choice made here.
- R-420 also removed render_contract Part 6's Optional Fourier block row, parked in `principia_00_philosophy.md`
  §7.6a; nothing here builds it.
