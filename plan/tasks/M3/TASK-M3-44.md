# TASK-M3-44 — The syzygy count against the atlas's `syz` on every row (OC-5), and its counting convention

- **Milestone:** M3
- **Closes:** REQ-VAL-220, REQ-VAL-221
- **Depends on:** TASK-M3-16, TASK-M3-42
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~200 lines

## Goal
The replication suite's targets OC-5, OC-5's counting convention run as engine tests (`principia_dd_replication.md`; R-594, R-629). On every atlas row OC-1 integrates, the engine's syzygy count over one period must equal the catalogue's `syz` (OC-5). The convention by which a syzygy is counted over one period for OC-5 — in particular a syzygy at t = 0 and again at t = T, which every start on the Šuvakov plane has, being collinear (§ 4.3) — must be defined in principia_dd_replication.md § 5.2 by the task that needs it, read from the atlas's own counting where it can be, and reviewed by the physics reviewer before merge (R-72).

## References
- `docs/design/principia_dd_replication.md` § "5.2 The orbit atlas"
- `docs/design/principia_dd_replication.md` § "4.3 The atlas's slice families"
- `docs/design/principia_dd_replication.md` § "8.1 Carried over from the 25 Oct instruction"
- `docs/design/principia_dd_replication.md` § "8.2 Learnt since"
- `docs/design/principia_dd_replication.md` § "8.3 How the tasks apply them"
- `decisions.md` § "R-594 — The replication suite is adopted: published three-body results reproduced as engine tests in tiers A–D, ported as a design doc that extends the validation-orbits doc; each target a VAL requirement with its tolerance a calibration; every published table transcribed twice and never tuned to; sequenced by need; the questions it lists queued *(changes no ruling or requirement in force; recorded with R-629's port)*"
- `decisions.md` § "R-629 — PLAN.md is adopted as the single plan for verification and replication, superseding as the working plan the 25 Oct instruction's tier tables, sequencing and fetch list; its seven draft requirements are ported as M3 requirements with their tolerances as calibrations; R-594's port follows it, with the orbit catalogue's six tests as tasks; the plan's other questions stay queued *(refines R-594; answers PLAN.md § 9 questions 1 and 2)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/notes/principia_validation_ground_truth_note.md` § "Four tiers of ground truth (increasing in what they prove)"
- `docs/notes/principia_validation_ground_truth_note.md` § "Chaos forbids pointwise comparison (exact vs structural)"

## Deliverables
- The counting convention written into `docs/design/principia_dd_replication.md` § 5.2 (R-72), with a "Removed lines" note.
- `crates/validation/src/gates/atlas_syzygy.rs`: the count per row from the crossing counter against `syz`.

## Acceptance tests
- `cargo xtask gate replication --target OC-5` — per row the crossing counter's count of syzygies over one period, by the convention REQ-VAL-221 defines, against `syz`; the comparison is of integers, exact; a row whose count differs is listed with both numbers and queued unless the cause is a located bug (rule 4); control: a run to 0.5·T must fail on every row with syz > 1 (REQ-VAL-220).
- Doc review (physics): `principia_dd_replication.md` § 5.2 states which crossings count as a syzygy, how one at t = 0 and at t = T counts, and how the count is read from the crossing counter, checked against a sample of rows' `syz` before the comparison; the physics reviewer approved (REQ-VAL-221).
- `scripts/check_no_atlas_params.sh` — prints `ok`: `atlas-params.json` is not committed, and nothing in this task reads it.

## Notes
- The atlas data are used under CC BY 4.0, credited to https://www.threebodyorbits.com with `docs/incoming/2026-10-26_orbit_catalogues/CITATIONS.md`'s per-family credits, on every fixture, export and figure. `atlas-params.json` never enters the repo (`scripts/check_no_atlas_params.sh`); no task may commit it.
- One of the orbit catalogue's six tests (`docs/incoming/2026-10-26_orbit_catalogues/INSTRUCTION_orbit_catalogues_2026-10-26.md` § "What to do with it (an addition to tier A)"), folded in by R-629.
- Reference data are read from the records named in each requirement's verify; the records stay records (R-159), and the contract is `principia_dd_replication.md`.
