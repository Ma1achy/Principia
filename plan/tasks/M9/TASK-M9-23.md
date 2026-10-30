# TASK-M9-23 — Binary-plus-single targets: Trani et al.'s planar row, Manwadkar et al.'s escape probabilities, Stone & Leigh's closed forms (TR-1, MK-1, SL-1)

- **Milestone:** M9
- **Closes:** REQ-VAL-318, REQ-VAL-319, REQ-VAL-320, REQ-VAL-321, REQ-VAL-322
- **Depends on:** TASK-M3-34
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-2, PIT-3
- **Size:** ~350 lines

## Goal
The replication suite's targets TR-1, MK-1, SL-1 run as engine tests (`principia_dd_replication.md`; R-594, R-629). The inclination-0 row of Trani et al. 2024's map, as a one-dimensional scan in binary phase, must reproduce the escaper sequence and the regularity statistic f_col (TR-1). Per-body escape probabilities over Manwadkar et al. 2021's eight mass sets must be measured in a planar re-run and compared as distributions with their Table 1 (MK-1). Binned binary energy and eccentricity distributions must be compared with Stone & Leigh 2019's planar closed forms, with the cut N_scramble ≥ 2 (SL-1). Each tolerance is a calibration, proposed here with its evidence and confirmed by the human with the M9 report (R-71).

## References
- `docs/design/principia_dd_replication.md` § "5.12 Targets that wait for a chart, a diagnostic or data"
- `docs/design/principia_dd_replication.md` § "8.1 Carried over from the 25 Oct instruction"
- `docs/design/principia_dd_replication.md` § "8.2 Learnt since"
- `docs/design/principia_dd_replication.md` § "8.3 How the tasks apply them"
- `decisions.md` § "R-594 — The replication suite is adopted: published three-body results reproduced as engine tests in tiers A–D, ported as a design doc that extends the validation-orbits doc; each target a VAL requirement with its tolerance a calibration; every published table transcribed twice and never tuned to; sequenced by need; the questions it lists queued *(changes no ruling or requirement in force; recorded with R-629's port)*"
- `decisions.md` § "R-629 — PLAN.md is adopted as the single plan for verification and replication, superseding as the working plan the 25 Oct instruction's tier tables, sequencing and fetch list; its seven draft requirements are ported as M3 requirements with their tolerances as calibrations; R-594's port follows it, with the orbit catalogue's six tests as tasks; the plan's other questions stay queued *(refines R-594; answers PLAN.md § 9 questions 1 and 2)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/notes/principia_validation_ground_truth_note.md` § "Four tiers of ground truth (increasing in what they prove)"
- `docs/notes/principia_validation_ground_truth_note.md` § "Chaos forbids pointwise comparison (exact vs structural)"

## Deliverables
- The work on the chart and diagnostics RQ-593 and RQ-596 rule; no chart is built here ahead of that ruling.
- `crates/validation/src/gates/binary_single.rs`; the calibration proposals.

## Acceptance tests
- `cargo xtask gate replication --target TR-1 --render` — the figure beside the paper's; the scan on a binary-plus-single chart once RQ-593 is ruled, f_col once RQ-596 is; reviewed by the physics reviewer against the figure; control: a scan at a different binary eccentricity must differ; the physics reviewer compares them; the outcome recorded with its tier: PASS where the comparison above holds, otherwise EXPLAINED DIVERGENCE (with the measurement showing the mechanism) or UNEXPLAINED, never a merge condition (R-649); the control still has to fire (REQ-VAL-318).
- `cargo xtask gate replication --target MK-1` — Table 1 transcribed twice (rule 2); a statistic at REQ-VAL-320's level; their runs are 3D, so the comparison is of a different experiment, which the report states (RQ-593); control: masses relabelled must move the probabilities; the outcome recorded with its tier: PASS where the comparison above holds, otherwise EXPLAINED DIVERGENCE (with the measurement showing the mechanism) or UNEXPLAINED, never a merge condition (R-649); the control still has to fire (REQ-VAL-319).
- `cargo xtask gate replication --target MK-1 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human with the M9 report and recorded in `decisions.md` (REQ-VAL-320).
- `cargo xtask gate replication --target SL-1` — the closed forms transcribed (rule 2); N_scramble once RQ-596 is ruled; a statistic at REQ-VAL-322's level; control: the 3D forms must fail; the outcome recorded with its tier: PASS where the comparison above holds, otherwise EXPLAINED DIVERGENCE (with the measurement showing the mechanism) or UNEXPLAINED, never a merge condition (R-649); the control still has to fire (REQ-VAL-321).
- `cargo xtask gate replication --target SL-1 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human with the M9 report and recorded in `decisions.md` (REQ-VAL-322).
- `git diff --no-index` of each fixture table's two transcriptions — empty, or each difference settled against the page and recorded beside the fixture, before the fixture is committed (R-594 item 4).

## Notes
- Calibrations proposed here (R-71; human confirmation with the M9 report, then recorded in decisions.md): REQ-VAL-320, REQ-VAL-322.
- Waits on RQ-593, RQ-596: the task starts only once they are ruled, and builds no chart or diagnostic ahead of the ruling (R-594 item 5).
- R-594 item 4: every published table this task commits as a fixture is transcribed twice, independently, and the two copies are diffed before the commit, each row recording its page; both copies and the diff stay beside the fixture. A table a record has already transcribed twice and diffed is copied with its sha256 and the record's diff cited. Never tune to match: a disagreement with the paper goes to `REVIEW_QUEUE.md` with both numbers, as RQ-570 did, and the fixture is not edited.
- Reference data are read from the records named in each requirement's verify; the records stay records (R-159), and the contract is `principia_dd_replication.md`.
- *30 Oct 2026 (R-649, R-650):* moved to M9, external replication, and renumbered: formerly TASK-M3-63 (milestone M3). Its targets are measurements, never gates: each is scored in M9, or early as a peek (information only, `docs/verification/peek_log.md`), and its outcome is PASS, EXPLAINED DIVERGENCE (with the measurement showing the mechanism) or UNEXPLAINED, never a failure that blocks; like-for-like is checked first, and nothing is tuned, loosened or special-cased to pass (`docs/design/principia_dd_replication.md` § "12.3 Outcomes, the like-for-like check and the forbidden moves"). The tier each target binds (§ "12.2 Tiers"; a calibration takes its target's): TR-1 (REQ-VAL-318): tier (a); MK-1 (REQ-VAL-319): tier (a); SL-1 (REQ-VAL-321): tier (a). *Was:* "The tier each binds is § '12.2 Tiers''s: (b); (a) statistically or qualitatively." (corrected 30 Oct 2026 to the owner's confirmation of the tier reading, R-649). Its calibrations are confirmed by the human with the M9 report (REQ-VAL-340), *was:* "at the M3 gate". A wait on a ruling or data below holds this task's scoring, never a milestone.
