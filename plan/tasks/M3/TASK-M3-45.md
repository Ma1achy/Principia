# TASK-M3-45 — The atlas's per-family coverage report (OC-6)

- **Milestone:** M3
- **Closes:** REQ-VAL-222
- **Depends on:** TASK-M3-42, TASK-M3-43, TASK-M3-44
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~150 lines

## Goal
The replication suite's targets OC-6 run as engine tests (`principia_dd_replication.md`; R-594, R-629). A per-family coverage report must print, for every atlas family, the rows tested, the closure order achieved and the failures, from OC-1's, OC-3's and OC-5's runs; a family in which every row fails must be investigated, and its cause located, before the engine is blamed (OC-6).

## References
- `docs/design/principia_dd_replication.md` § "5.2 The orbit atlas"
- `docs/design/principia_dd_replication.md` § "8.1 Carried over from the 25 Oct instruction"
- `docs/design/principia_dd_replication.md` § "8.2 Learnt since"
- `docs/design/principia_dd_replication.md` § "8.3 How the tasks apply them"
- `decisions.md` § "R-594 — The replication suite is adopted: published three-body results reproduced as engine tests in tiers A–D, ported as a design doc that extends the validation-orbits doc; each target a VAL requirement with its tolerance a calibration; every published table transcribed twice and never tuned to; sequenced by need; the questions it lists queued *(changes no ruling or requirement in force; recorded with R-629's port)*"
- `decisions.md` § "R-629 — PLAN.md is adopted as the single plan for verification and replication, superseding as the working plan the 25 Oct instruction's tier tables, sequencing and fetch list; its seven draft requirements are ported as M3 requirements with their tolerances as calibrations; R-594's port follows it, with the orbit catalogue's six tests as tasks; the plan's other questions stay queued *(refines R-594; answers PLAN.md § 9 questions 1 and 2)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/notes/principia_validation_ground_truth_note.md` § "Four tiers of ground truth (increasing in what they prove)"
- `docs/notes/principia_validation_ground_truth_note.md` § "Chaos forbids pointwise comparison (exact vs structural)"

## Deliverables
- `cargo xtask gate atlas-coverage`: the report per family from OC-1's, OC-3's and OC-5's runs, the skipped rows named.

## Acceptance tests
- `cargo xtask gate replication --target OC-6` — cargo xtask gate atlas-coverage writes the report; every one of the 41 families appears, with the rows skipped for T > 200 (RQ-592) counted and named; control: a family run with a deliberately permuted mass order must read as failing every row (REQ-VAL-222).
- `scripts/check_no_atlas_params.sh` — prints `ok`: `atlas-params.json` is not committed, and nothing in this task reads it.

## Notes
- The atlas data are used under CC BY 4.0, credited to https://www.threebodyorbits.com with `docs/incoming/2026-10-26_orbit_catalogues/CITATIONS.md`'s per-family credits, on every fixture, export and figure. `atlas-params.json` never enters the repo (`scripts/check_no_atlas_params.sh`); no task may commit it.
- 667 atlas rows have T > 200, past R-610's cap; they are skipped and counted while RQ-592 is open.
- One of the orbit catalogue's six tests (`docs/incoming/2026-10-26_orbit_catalogues/INSTRUCTION_orbit_catalogues_2026-10-26.md` § "What to do with it (an addition to tier A)"), folded in by R-629.
- Reference data are read from the records named in each requirement's verify; the records stay records (R-159), and the contract is `principia_dd_replication.md`.
