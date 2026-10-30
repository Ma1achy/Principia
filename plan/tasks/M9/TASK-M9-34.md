# TASK-M9-34 — The atlas's free-fall orbits marked on the closure field, one plane per mass set (AT-D2)

- **Milestone:** M9
- **Closes:** REQ-VAL-308
- **Depends on:** TASK-M9-06, TASK-M8-74, TASK-M4-15
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~200 lines

## Goal
The replication suite's targets AT-D2 run as engine tests (`principia_dd_replication.md`; R-594, R-629). A Result of the closure field on the latent chart's `z_q = 0` plane must mark the atlas's 311 free-fall orbits on the nine planes of their nine mass sets, and each must sit at low closure by REQ-VAL-137's threshold (AT-D2).

## References
- `docs/design/principia_dd_replication.md` § "5.2 The orbit atlas"
- `docs/design/principia_dd_replication.md` § "8.1 Carried over from the 25 Oct instruction"
- `docs/design/principia_dd_replication.md` § "8.2 Learnt since"
- `docs/design/principia_dd_replication.md` § "8.3 How the tasks apply them"
- `decisions.md` § "R-594 — The replication suite is adopted: published three-body results reproduced as engine tests in tiers A–D, ported as a design doc that extends the validation-orbits doc; each target a VAL requirement with its tolerance a calibration; every published table transcribed twice and never tuned to; sequenced by need; the questions it lists queued *(changes no ruling or requirement in force; recorded with R-629's port)*"
- `decisions.md` § "R-629 — PLAN.md is adopted as the single plan for verification and replication, superseding as the working plan the 25 Oct instruction's tier tables, sequencing and fetch list; its seven draft requirements are ported as M3 requirements with their tolerances as calibrations; R-594's port follows it, with the orbit catalogue's six tests as tasks; the plan's other questions stay queued *(refines R-594; answers PLAN.md § 9 questions 1 and 2)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/design/principia_dd_validation_orbits.md` § "6.1 Why rendering beats searching"
- `docs/notes/principia_validation_ground_truth_note.md` § "Four tiers of ground truth (increasing in what they prove)"
- `docs/notes/principia_validation_ground_truth_note.md` § "Chaos forbids pointwise comparison (exact vs structural)"

## Deliverables
- One Result per mass set on the latent `z_q = 0` plane with the 311 orbits marked, the atlas's credit line on each figure.

## Acceptance tests
- `cargo xtask gate replication --target AT-D2` — one Result per mass set (R-572), f64, uniform; each orbit's pixel against REQ-VAL-137's threshold; the figure carries the atlas's credit line; control: the markers moved by a pixel row must miss the low closure; the outcome recorded with its tier: PASS where the comparison above holds, otherwise EXPLAINED DIVERGENCE (with the measurement showing the mechanism) or UNEXPLAINED, never a merge condition (R-649); the control still has to fire (REQ-VAL-308).
- `scripts/check_no_atlas_params.sh` — prints `ok`: `atlas-params.json` is not committed, and nothing in this task reads it.

## Notes
- The atlas data are used under CC BY 4.0, credited to https://www.threebodyorbits.com with `docs/incoming/2026-10-26_orbit_catalogues/CITATIONS.md`'s per-family credits, on every fixture, export and figure. `atlas-params.json` never enters the repo (`scripts/check_no_atlas_params.sh`); no task may commit it.
- Reference data are read from the records named in each requirement's verify; the records stay records (R-159), and the contract is `principia_dd_replication.md`.
- *30 Oct 2026 (R-649, R-650):* moved to M9, external replication, and renumbered: formerly TASK-M8-98 (milestone M8). Its targets are measurements, never gates: each is scored in M9, or early as a peek (information only, `docs/verification/peek_log.md`), and its outcome is PASS, EXPLAINED DIVERGENCE (with the measurement showing the mechanism) or UNEXPLAINED, never a failure that blocks; like-for-like is checked first, and nothing is tuned, loosened or special-cased to pass (`docs/design/principia_dd_replication.md` § "12.3 Outcomes, the like-for-like check and the forbidden moves"). The tier each target binds (§ "12.2 Tiers"; a calibration takes its target's): AT-D2 (REQ-VAL-308): tier (a), its scoring on an f64 Result waits on RQ-621. *Was:* "The tier each binds is § '12.2 Tiers''s: (b); (a) statistically or qualitatively." (corrected 30 Oct 2026 to the owner's confirmation of the tier reading, R-649).
