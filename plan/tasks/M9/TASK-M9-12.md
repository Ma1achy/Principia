# TASK-M9-12 — The butterflies' two published periods, decided by closure (BG-4, the first batch's DRAFT-VAL-07)

- **Milestone:** M9
- **Closes:** REQ-VAL-213
- **Depends on:** TASK-M3-24, TASK-M3-08
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~200 lines

## Goal
The replication suite's targets BG-4 run as engine tests (`principia_dd_replication.md`; R-594, R-629). From the Belgrade gallery's (p₁, p₂) for butterfly I (I.2.A) and butterfly II (I.2.B), the time of the return-proximity minimum near one period must be measured under the default occupant and recorded against both published periods (6.235641 and 6.234671; 7.003907 and 7.003707), and against the atlas's refined periods; the gallery CSV is not edited.

## References
- `docs/design/principia_dd_replication.md` § "5.4 The Belgrade orbits"
- `docs/design/principia_dd_replication.md` § "6. The first batch"
- `docs/design/principia_dd_replication.md` § "8.1 Carried over from the 25 Oct instruction"
- `docs/design/principia_dd_replication.md` § "8.2 Learnt since"
- `docs/design/principia_dd_replication.md` § "8.3 How the tasks apply them"
- `decisions.md` § "R-594 — The replication suite is adopted: published three-body results reproduced as engine tests in tiers A–D, ported as a design doc that extends the validation-orbits doc; each target a VAL requirement with its tolerance a calibration; every published table transcribed twice and never tuned to; sequenced by need; the questions it lists queued *(changes no ruling or requirement in force; recorded with R-629's port)*"
- `decisions.md` § "R-629 — PLAN.md is adopted as the single plan for verification and replication, superseding as the working plan the 25 Oct instruction's tier tables, sequencing and fetch list; its seven draft requirements are ported as M3 requirements with their tolerances as calibrations; R-594's port follows it, with the orbit catalogue's six tests as tasks; the plan's other questions stay queued *(refines R-594; answers PLAN.md § 9 questions 1 and 2)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/notes/principia_validation_ground_truth_note.md` § "Four tiers of ground truth (increasing in what they prove)"
- `docs/notes/principia_validation_ground_truth_note.md` § "Chaos forbids pointwise comparison (exact vs structural)"

## Deliverables
- `crates/validation/src/gates/butterfly_periods.rs`: the return-proximity minimum near one period with its propagated uncertainty, against both published periods and the atlas's refined ones.
- The result added to RQ-576's record in `REVIEW_QUEUE.md`; the gallery CSV not edited.

## Acceptance tests
- `cargo xtask gate replication --target BG-4` — the minimum's time with its uncertainty propagated from the six-decimal (p₁, p₂) and the eta ladder; the test asserts the uncertainty is smaller than the published periods' difference (about 1e-3 for I, 2e-4 for II) before it reads which period is nearer; the result goes to RQ-576's record (REQ-VAL-213).
- `scripts/check_no_atlas_params.sh` — prints `ok`: `atlas-params.json` is not committed, and nothing in this task reads it.

## Notes
- The atlas data are used under CC BY 4.0, credited to https://www.threebodyorbits.com with `docs/incoming/2026-10-26_orbit_catalogues/CITATIONS.md`'s per-family credits, on every fixture, export and figure. `atlas-params.json` never enters the repo (`scripts/check_no_atlas_params.sh`); no task may commit it.
- Part of the first batch (`principia_dd_replication.md` § 6), ported from PLAN.md § 6's drafts by R-629.
- Reference data are read from the records named in each requirement's verify; the records stay records (R-159), and the contract is `principia_dd_replication.md`.
- *30 Oct 2026 (R-649, R-650):* moved to M9, external replication, and renumbered: formerly TASK-M3-52 (milestone M3). Its targets are measurements, never gates: each is scored in M9, or early as a peek (information only, `docs/verification/peek_log.md`), and its outcome is PASS, EXPLAINED DIVERGENCE (with the measurement showing the mechanism) or UNEXPLAINED, never a failure that blocks; like-for-like is checked first, and nothing is tuned, loosened or special-cased to pass (`docs/design/principia_dd_replication.md` § "12.3 Outcomes, the like-for-like check and the forbidden moves"). The tier each binds is § "12.2 Tiers"'s: (b).
