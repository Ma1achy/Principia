# TASK-M9-33 — Stability as structure: atlas orbits off the Šuvakov plane against their published stability (OC-4, AT-D5)

- **Milestone:** M9
- **Closes:** REQ-VAL-309, REQ-VAL-310
- **Depends on:** TASK-M9-05, TASK-M8-74, TASK-M6-05
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~350 lines

## Goal
The replication suite's targets OC-4 run as engine tests (`principia_dd_replication.md`; R-594, R-629). On a chart through each atlas orbit not on the Šuvakov plane, the payload's leaf decisions, spread and `error_ratio` must place linearly stable orbits inside resolved regular islands and unstable ones on boundaries, compared with the catalogue's `stab` and `stable_pub` and their sources as an agreement table with every disagreement listed, with the per-orbit neighbourhood image (OC-4, AT-D5). Each tolerance is a calibration, proposed here with its evidence and confirmed by the human with the M9 report (R-71).

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
- `decisions.md` § "R-653 — Five map targets and B-2 are scored on tier (a), their f64 Result or CPU FTLE run beside it and recorded as information, an explained divergence between the two being a precision-tier finding *(closes RQ-622; refines R-649)*"

## Deliverables
- Result runs through each orbit, at f32 (what scores: the GPU explorer's f32 output) and at f64 (beside it, as information); the agreement table with every disagreement listed; the per-orbit neighbourhood images with the atlas's credit line; the calibration proposal.

## Acceptance tests
- `cargo xtask gate replication --target OC-4` — scored on tier (a): the GPU explorer's f32 output; the island criterion read from the payload; agreement at REQ-VAL-310's level; every disagreement listed with both labels and its source; the f64 Result (R-572) run beside it and recorded as information, a divergence between the two a precision-tier finding; control: labels shuffled between orbits must lower the agreement to chance; the outcome recorded with its tier: PASS where the comparison above holds, otherwise EXPLAINED DIVERGENCE (with the measurement showing the mechanism) or UNEXPLAINED, never a merge condition (R-649); the control still has to fire (REQ-VAL-309).
- `cargo xtask gate replication --target OC-4 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human with the M9 report and recorded in `decisions.md` (REQ-VAL-310).
- `scripts/check_no_atlas_params.sh` — prints `ok`: `atlas-params.json` is not committed, and nothing in this task reads it.

## Notes
- Calibrations proposed here (R-71; human confirmation with the M9 report, then recorded in decisions.md): REQ-VAL-310.
- The atlas data are used under CC BY 4.0, credited to https://www.threebodyorbits.com with `docs/incoming/2026-10-26_orbit_catalogues/CITATIONS.md`'s per-family credits, on every fixture, export and figure. `atlas-params.json` never enters the repo (`scripts/check_no_atlas_params.sh`); no task may commit it.
- One of the orbit catalogue's six tests (`docs/incoming/2026-10-26_orbit_catalogues/INSTRUCTION_orbit_catalogues_2026-10-26.md` § "What to do with it (an addition to tier A)"), folded in by R-629.
- Reference data are read from the records named in each requirement's verify; the records stay records (R-159), and the contract is `principia_dd_replication.md`.
- *30 Oct 2026 (R-649, R-650):* moved to M9, external replication, and renumbered: formerly TASK-M8-97 (milestone M8). Its targets are measurements, never gates: each is scored in M9, or early as a peek (information only, `docs/verification/peek_log.md`), and its outcome is PASS, EXPLAINED DIVERGENCE (with the measurement showing the mechanism) or UNEXPLAINED, never a failure that blocks; like-for-like is checked first, and nothing is tuned, loosened or special-cased to pass (`docs/design/principia_dd_replication.md` § "12.3 Outcomes, the like-for-like check and the forbidden moves"). The tier each target binds (§ "12.2 Tiers"; a calibration takes its target's): OC-4 (off the Šuvakov plane) (REQ-VAL-309): tier (a) (R-653, RQ-622 (c): scored on tier (a), the f64 Result beside it as information; *was:* "its scoring on an f64 Result waits on RQ-622"). *Was:* "The tier each binds is § '12.2 Tiers''s: (b); (a) statistically or qualitatively." (corrected 30 Oct 2026 to the owner's confirmation of the tier reading, R-649). Its calibrations are confirmed by the human with the M9 report (REQ-VAL-340), *was:* "at the M8 gate".
- *1 Nov 2026 (R-653; code review 5730587922 of PR #375, C3; applied per R-369):* the GPU explorer's f32 output this task scores is a Result at f32: the survey kernel on the GPU, TASK-M4-17's uniform path, as TASK-M8-74 builds it. TASK-M8-74 therefore needs no explorer dependency beside it. The f64 Result is the same job at f64.
