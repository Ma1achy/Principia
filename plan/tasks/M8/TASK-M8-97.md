# TASK-M8-97 — Stability as structure: atlas orbits off the Šuvakov plane against their published stability (OC-4, AT-D5)

- **Milestone:** M8
- **Closes:** REQ-VAL-309, REQ-VAL-310
- **Depends on:** TASK-M8-74, TASK-M6-05, TASK-M3-45
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~350 lines

## Goal
The replication suite's targets OC-4 run as engine tests (`principia_dd_replication.md`; R-594, R-629). On a chart through each atlas orbit not on the Šuvakov plane, the payload's leaf decisions, spread and `error_ratio` must place linearly stable orbits inside resolved regular islands and unstable ones on boundaries, compared with the catalogue's `stab` and `stable_pub` and their sources as an agreement table with every disagreement listed, with the per-orbit neighbourhood image (OC-4, AT-D5). Each tolerance is a calibration, proposed here with its evidence and confirmed by the human at the M8 gate (R-71).

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
- Result runs through each orbit; the agreement table with every disagreement listed; the per-orbit neighbourhood images with the atlas's credit line; the calibration proposal.

## Acceptance tests
- `cargo xtask gate replication --target OC-4` — Result runs (R-572) at f64; the island criterion read from the payload; agreement at REQ-VAL-310's level; every disagreement listed with both labels and its source; control: labels shuffled between orbits must lower the agreement to chance (REQ-VAL-309).
- `cargo xtask gate replication --target OC-4 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human at the M8 gate and recorded in `decisions.md` (REQ-VAL-310).
- `scripts/check_no_atlas_params.sh` — prints `ok`: `atlas-params.json` is not committed, and nothing in this task reads it.

## Notes
- Calibrations proposed here (R-71; human confirmation at the M8 gate, then recorded in decisions.md): REQ-VAL-310.
- The atlas data are used under CC BY 4.0, credited to https://www.threebodyorbits.com with `docs/incoming/2026-10-26_orbit_catalogues/CITATIONS.md`'s per-family credits, on every fixture, export and figure. `atlas-params.json` never enters the repo (`scripts/check_no_atlas_params.sh`); no task may commit it.
- One of the orbit catalogue's six tests (`docs/incoming/2026-10-26_orbit_catalogues/INSTRUCTION_orbit_catalogues_2026-10-26.md` § "What to do with it (an addition to tier A)"), folded in by R-629.
- Reference data are read from the records named in each requirement's verify; the records stay records (R-159), and the contract is `principia_dd_replication.md`.
