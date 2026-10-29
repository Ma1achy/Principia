# TASK-M3-43 — Published against refined: each atlas source's published initial conditions measured (OC-3)

- **Milestone:** M3
- **Closes:** REQ-VAL-218, REQ-VAL-219
- **Depends on:** TASK-M3-42
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~200 lines

## Goal
The replication suite's targets OC-3 run as engine tests (`principia_dd_replication.md`; R-594, R-629). For every atlas row it runs, our closure after one period integrated from the published initial conditions (`pub_*`) must be measured and recorded beside the row's `closure_published`, as a measurement of each source's published initial conditions; for the five Li & Liao 2017 rows whose high-precision values are Hristov et al.'s, the closure from the 10-digit published values must be recorded beside the high-precision record's agreement figures (OC-3). Each tolerance is a calibration, proposed here with its evidence and confirmed by the human at the M3 gate (R-71).

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
- `crates/validation/src/gates/atlas_published.rs`: our closure from `pub_*` beside `closure_published`, every row outside the band listed with both numbers; Li & Liao 2017's five rows beside `docs/incoming/2026-10-27_hp_ics/agreement.txt`'s figures.
- The calibration proposal for the agreement band.

## Acceptance tests
- `cargo xtask gate replication --target OC-3` — per row our closure from pub_* under the default occupant at f64 beside closure_published; rows within REQ-VAL-219's band counted, every row outside it listed with both numbers, never failed (the orbit-catalogue instruction's item 3); rows with T > 200 recorded as "not evaluated within cap", never failed (R-636); control: on a row whose catalogued published closure exceeds 1e-6, the run from ref_* must close better than the run from pub_* (REQ-VAL-218).
- `cargo xtask gate replication --target OC-3 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human at the M3 gate and recorded in `decisions.md` (REQ-VAL-219).
- `scripts/check_no_atlas_params.sh` — prints `ok`: `atlas-params.json` is not committed, and nothing in this task reads it.

## Notes
- Calibrations proposed here (R-71; human confirmation at the M3 gate, then recorded in decisions.md): REQ-VAL-219.
- The atlas data are used under CC BY 4.0, credited to https://www.threebodyorbits.com with `docs/incoming/2026-10-26_orbit_catalogues/CITATIONS.md`'s per-family credits, on every fixture, export and figure. `atlas-params.json` never enters the repo (`scripts/check_no_atlas_params.sh`); no task may commit it.
- 667 atlas rows have T > 200, past R-610's cap; they are recorded as "not evaluated within cap", counted and named, never as failures (R-636, closing RQ-592); TASK-M4-26 runs them at a raised cap (REQ-VAL-332). *Was:* "they are skipped and counted while RQ-592 is open."
- One of the orbit catalogue's six tests (`docs/incoming/2026-10-26_orbit_catalogues/INSTRUCTION_orbit_catalogues_2026-10-26.md` § "What to do with it (an addition to tier A)"), folded in by R-629.
- Reference data are read from the records named in each requirement's verify; the records stay records (R-159), and the contract is `principia_dd_replication.md`.
