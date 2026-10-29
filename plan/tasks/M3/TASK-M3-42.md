# TASK-M3-42 — Atlas closure at order (OC-1, the first batch's DRAFT-VAL-01)

- **Milestone:** M3
- **Closes:** REQ-VAL-201, REQ-VAL-202
- **Depends on:** TASK-M3-24, TASK-M3-08, TASK-M2-51
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~300 lines

## Goal
The replication suite's targets OC-1 run as engine tests (`principia_dd_replication.md`; R-594, R-629). Every Three Body Orbits atlas row whose catalogued refined closure is below 1e-9 must return to its initial state after one period T when integrated from its f64 refined initial conditions (`ref_*`) under the default occupant (Heggie × RK4, R-560) and under Aarseth–Zare, at each occupant's stated order across an eta ladder; rows from the site's own searches are closure tests only and never cited (rule 13). Each tolerance is a calibration, proposed here with its evidence and confirmed by the human at the M3 gate (R-71).

## References
- `docs/design/principia_dd_replication.md` § "5.2 The orbit atlas"
- `docs/design/principia_dd_replication.md` § "6. The first batch"
- `docs/design/principia_dd_replication.md` § "8.1 Carried over from the 25 Oct instruction"
- `docs/design/principia_dd_replication.md` § "8.2 Learnt since"
- `docs/design/principia_dd_replication.md` § "8.3 How the tasks apply them"
- `decisions.md` § "R-594 — The replication suite is adopted: published three-body results reproduced as engine tests in tiers A–D, ported as a design doc that extends the validation-orbits doc; each target a VAL requirement with its tolerance a calibration; every published table transcribed twice and never tuned to; sequenced by need; the questions it lists queued *(changes no ruling or requirement in force; recorded with R-629's port)*"
- `decisions.md` § "R-629 — PLAN.md is adopted as the single plan for verification and replication, superseding as the working plan the 25 Oct instruction's tier tables, sequencing and fetch list; its seven draft requirements are ported as M3 requirements with their tolerances as calibrations; R-594's port follows it, with the orbit catalogue's six tests as tasks; the plan's other questions stay queued *(refines R-594; answers PLAN.md § 9 questions 1 and 2)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/design/principia_dd_validation_orbits.md` § "3. Proposed suite"
- `docs/notes/principia_validation_ground_truth_note.md` § "Four tiers of ground truth (increasing in what they prove)"
- `docs/notes/principia_validation_ground_truth_note.md` § "Chaos forbids pointwise comparison (exact vs structural)"

## Deliverables
- `crates/validation/src/gates/atlas_closure.rs`: one period from `ref_*` per row under the default occupant and under Aarseth–Zare across the eta ladder, the fitted order, short periods first and the smallest-`rmin` rows on purpose; rows with T > 200 recorded as "not evaluated within cap", counted and named, never as failures (R-636; *was:* "skipped and counted while RQ-592 is open").
- The per-row amplification measurement and the calibration proposal for the closure tolerance.

## Acceptance tests
- `cargo xtask gate replication --target OC-1` — reference data docs/incoming/2026-10-26_orbit_catalogues/threebodyorbits_ics.csv (sha256 52a67e5d…), short periods first, the smallest-rmin rows included on purpose; per row |state(T) − state(0)| at each eta and the fitted order; a row passes when its closure sits within REQ-VAL-202's tolerance of its floor and its order matches the occupant's; control: a row integrated to 0.9·T must fail; every figure carries the CC BY 4.0 credit line of CITATIONS.md; the 667 rows with T > 200, past R-610's cap, are recorded as "not evaluated within cap", counted and named, never as failures (R-636); TASK-M4-26 runs them at a raised cap (REQ-VAL-332) (REQ-VAL-201).
- `cargo xtask gate replication --target OC-1 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human at the M3 gate and recorded in `decisions.md` (REQ-VAL-202).
- `scripts/check_no_atlas_params.sh` — prints `ok`: `atlas-params.json` is not committed, and nothing in this task reads it.

## Notes
- Calibrations proposed here (R-71; human confirmation at the M3 gate, then recorded in decisions.md): REQ-VAL-202.
- The atlas data are used under CC BY 4.0, credited to https://www.threebodyorbits.com with `docs/incoming/2026-10-26_orbit_catalogues/CITATIONS.md`'s per-family credits, on every fixture, export and figure. `atlas-params.json` never enters the repo (`scripts/check_no_atlas_params.sh`); no task may commit it.
- 667 atlas rows have T > 200, past R-610's cap; they are recorded as "not evaluated within cap", counted and named, never as failures (R-636, closing RQ-592); TASK-M4-26 runs them at a raised cap (REQ-VAL-332). *Was:* "they are skipped and counted while RQ-592 is open."
- One of the orbit catalogue's six tests (`docs/incoming/2026-10-26_orbit_catalogues/INSTRUCTION_orbit_catalogues_2026-10-26.md` § "What to do with it (an addition to tier A)"), folded in by R-629.
- Part of the first batch (`principia_dd_replication.md` § 6), ported from PLAN.md § 6's drafts by R-629.
- Reference data are read from the records named in each requirement's verify; the records stay records (R-159), and the contract is `principia_dd_replication.md`.
