# TASK-M9-11 — The no-integration controls: Monaghan's planar eccentricity row and the random-triangle Z (Q-2's Theory row, Q-4; DRAFT-VAL-06)

- **Milestone:** M9
- **Closes:** REQ-VAL-211, REQ-VAL-212
- **Depends on:** TASK-M3-12, TASK-M3-24
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-2, PIT-3
- **Size:** ~200 lines

## Goal
The replication suite's targets Q-2's Theory row, Q-4 run as engine tests (`principia_dd_replication.md`; R-594, R-629). Two controls that need no integration: (i) Monaghan's planar eccentricity distribution f(E) = E/(1 − E²)^{1/2}, integrated over ten bins of width 0.1 and rounded half-up, must reproduce Aarseth et al. 1994b's Table II 'Theory' row exactly (1, 2, 3, 4, 5, 7, 9, 11, 16, 44 %; median 0.87); (ii) 2,500 triangles drawn uniformly in the unit disc, with Z computed by the validation crate's own Z, must give a distribution compatible with that paper's 17, 16, 14, 14, 12, 10, 8, 5, 4, 1 %. Each tolerance is a calibration, proposed here with its evidence and confirmed by the human with the M9 report (R-71).

## References
- `docs/design/principia_dd_replication.md` § "5.7 The Burrau neighbourhood: Aarseth et al. 1994a and 1994b"
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
- `fixtures/aarseth_1994b/table2_theory.csv` and the p. 135 Z bins, transcribed twice, the copies and their diff kept.
- `crates/validation/src/gates/outcome_controls.rs`: the Theory row exactly, the Z distribution at its calibrated level; the eccentricity and Z functions the outcome code's own.
- The calibration proposal for (ii)'s level.

## Acceptance tests
- `cargo xtask gate replication --target Q-4` — (i) exact; its negative control, the 3D thermal form f(E) = 2E, must fail; (ii) a χ² or KS statistic against the printed bins at REQ-VAL-212's level, with both readings of 'the maximum distance represents the binary' run and recorded; the eccentricity and Z functions are the ones the outcome code uses, not copies; each part's outcome recorded with its own tier, by § 12.2 ((b) for (i); (a) for (ii), which has no integration): PASS where that part's comparison above holds, otherwise EXPLAINED DIVERGENCE (with the measurement showing the mechanism) or UNEXPLAINED, never a merge condition (R-649), a part not yet scored reading "not scored" with what it waits on; what scores the (a) part in this task, which has no GPU run, is RQ-669; the control still has to fire (REQ-VAL-211).
- `cargo xtask gate replication --target Q-4 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human with the M9 report and recorded in `decisions.md` (REQ-VAL-212).
- `git diff --no-index` of each fixture table's two transcriptions — empty, or each difference settled against the page and recorded beside the fixture, before the fixture is committed (R-594 item 4).

## Notes
- Calibrations proposed here (R-71; human confirmation with the M9 report, then recorded in decisions.md): REQ-VAL-212.
- R-594 item 4: every published table this task commits as a fixture is transcribed twice, independently, and the two copies are diffed before the commit, each row recording its page; both copies and the diff stay beside the fixture. A table a record has already transcribed twice and diffed is copied with its sha256 and the record's diff cited. Never tune to match: a disagreement with the paper goes to `REVIEW_QUEUE.md` with both numbers, as RQ-570 did, and the fixture is not edited.
- Part of the first batch (`principia_dd_replication.md` § 6), ported from PLAN.md § 6's drafts by R-629.
- Reference data are read from the records named in each requirement's verify; the records stay records (R-159), and the contract is `principia_dd_replication.md`.
- *30 Oct 2026 (R-649, R-650):* moved to M9, external replication, and renumbered: formerly TASK-M3-51 (milestone M3). Its targets are measurements, never gates: each is scored in M9, or early as a peek (information only, `docs/verification/peek_log.md`), and its outcome is PASS, EXPLAINED DIVERGENCE (with the measurement showing the mechanism) or UNEXPLAINED, never a failure that blocks; like-for-like is checked first, and nothing is tuned, loosened or special-cased to pass (`docs/design/principia_dd_replication.md` § "12.3 Outcomes, the like-for-like check and the forbidden moves"). The tier each target binds (§ "12.2 Tiers"; a calibration takes its target's): Q-2's Theory row, Q-4 (REQ-VAL-211): tier (b) for (i), tier (a) for (ii). *Was:* "The tier each binds is § '12.2 Tiers''s: (b)." (corrected 30 Oct 2026 to the owner's confirmation of the tier reading, R-649). Its calibrations are confirmed by the human with the M9 report (REQ-VAL-340), *was:* "at the M3 gate".
- *1 Nov 2026 (physics review 5730823192 of PR #375, M2; applied per R-369):* Q-4 is a mixed target, scored on two tiers (`principia_dd_replication.md` § 12.2), so each part records its own outcome and TASK-M9-37 reads them part by part; neither part has a GPU run to miss (§ 12.2's row: "no integration"), and a single outcome would hide that (i) is exact and (ii) statistical. *Note was:* "a GPU miss on the (b) part is expected, not a defect, and a single outcome would hide which instrument agreed." (code review 5730886652 of PR #375, S2). *Was (acceptance):* "the outcome recorded with its tier: PASS where the comparison above holds, otherwise EXPLAINED DIVERGENCE …".
