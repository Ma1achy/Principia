# TASK-M9-18 — Aarseth et al. 1994b: triple approaches and parameter distributions over region D, circle R and the Pythagorean escapers (Q-1 to Q-3)

- **Milestone:** M9
- **Closes:** REQ-VAL-262, REQ-VAL-263, REQ-VAL-264, REQ-VAL-265, REQ-VAL-266, REQ-VAL-267
- **Depends on:** TASK-M9-11, TASK-M9-17
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-2, PIT-3
- **Size:** ~350 lines

## Goal
The replication suite's targets Q-1, Q-2, Q-3 run as engine tests (`principia_dd_replication.md`; R-594, R-629). Aarseth et al. 1994b's Table I triple-approach means over 2,500 region-D systems and 2,500 circle-R systems must be reproduced as distributions, with the paper's escape criterion (10d) recorded beside ours (rule 9) (Q-1). Aarseth et al. 1994b's Table II binned parameter distributions, all but the 'Theory' row (which the first batch's control checks), must be reproduced as distributions (Q-2). Aarseth et al. 1994b's Table III, regions D and R against the 831 Pythagorean escapers, must be reproduced as distributions with the units stated (1994b's τ against 1994a's T_cr = 3.84) (Q-3). Each tolerance is a calibration, proposed here with its evidence and confirmed by the human with the M9 report (R-71).

## References
- `docs/design/principia_dd_replication.md` § "5.7 The Burrau neighbourhood: Aarseth et al. 1994a and 1994b"
- `docs/design/principia_dd_replication.md` § "8.1 Carried over from the 25 Oct instruction"
- `docs/design/principia_dd_replication.md` § "8.2 Learnt since"
- `docs/design/principia_dd_replication.md` § "8.3 How the tasks apply them"
- `decisions.md` § "R-594 — The replication suite is adopted: published three-body results reproduced as engine tests in tiers A–D, ported as a design doc that extends the validation-orbits doc; each target a VAL requirement with its tolerance a calibration; every published table transcribed twice and never tuned to; sequenced by need; the questions it lists queued *(changes no ruling or requirement in force; recorded with R-629's port)*"
- `decisions.md` § "R-629 — PLAN.md is adopted as the single plan for verification and replication, superseding as the working plan the 25 Oct instruction's tier tables, sequencing and fetch list; its seven draft requirements are ported as M3 requirements with their tolerances as calibrations; R-594's port follows it, with the orbit catalogue's six tests as tasks; the plan's other questions stay queued *(refines R-594; answers PLAN.md § 9 questions 1 and 2)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/notes/principia_validation_ground_truth_note.md` § "Four tiers of ground truth (increasing in what they prove)"
- `docs/notes/principia_validation_ground_truth_note.md` § "Chaos forbids pointwise comparison (exact vs structural)"

## Deliverables
- `fixtures/aarseth_1994b/tables_i_iii.csv`, Tables I–III transcribed twice, the copies and their diff kept.
- Circle R's starts by Path B; the unit remap (τ against T_cr = 3.84) as a tested function; `crates/validation/src/gates/triple_approach.rs`.
- The calibration proposals.

## Acceptance tests
- `cargo xtask gate replication --target Q-1` — region D through the Anosova entry, circle R's starts injected by Path B (no chart); a statistic at REQ-VAL-263's level; control: region D's sample drawn from a disc of the wrong radius must fail (REQ-VAL-262).
- `cargo xtask gate replication --target Q-1 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human with the M9 report and recorded in `decisions.md` (REQ-VAL-263).
- `cargo xtask gate replication --target Q-2` — Table II as transcribed; the eccentricity and Z of the first batch's control; per column a statistic at REQ-VAL-265's level; control: a sample drawn at E > 0 must fail (REQ-VAL-264).
- `cargo xtask gate replication --target Q-2 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human with the M9 report and recorded in `decisions.md` (REQ-VAL-265).
- `cargo xtask gate replication --target Q-3` — the unit remap a tested function (rule 11); a statistic at REQ-VAL-267's level; control: the units left unconverted must fail (REQ-VAL-266).
- `cargo xtask gate replication --target Q-3 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human with the M9 report and recorded in `decisions.md` (REQ-VAL-267).
- `git diff --no-index` of each fixture table's two transcriptions — empty, or each difference settled against the page and recorded beside the fixture, before the fixture is committed (R-594 item 4).

## Notes
- Calibrations proposed here (R-71; human confirmation with the M9 report, then recorded in decisions.md): REQ-VAL-263, REQ-VAL-265, REQ-VAL-267.
- R-594 item 4: every published table this task commits as a fixture is transcribed twice, independently, and the two copies are diffed before the commit, each row recording its page; both copies and the diff stay beside the fixture. A table a record has already transcribed twice and diffed is copied with its sha256 and the record's diff cited. Never tune to match: a disagreement with the paper goes to `REVIEW_QUEUE.md` with both numbers, as RQ-570 did, and the fixture is not edited.
- Reference data are read from the records named in each requirement's verify; the records stay records (R-159), and the contract is `principia_dd_replication.md`.
- *30 Oct 2026 (R-649, R-650):* moved to M9, external replication, and renumbered: formerly TASK-M3-58 (milestone M3). Its targets are measurements, never gates: each is scored in M9, or early as a peek (information only, `docs/verification/peek_log.md`), and its outcome is PASS, EXPLAINED DIVERGENCE (with the measurement showing the mechanism) or UNEXPLAINED, never a failure that blocks; like-for-like is checked first, and nothing is tuned, loosened or special-cased to pass (`docs/design/principia_dd_replication.md` § "12.3 Outcomes, the like-for-like check and the forbidden moves"). The tier each target binds (§ "12.2 Tiers"; a calibration takes its target's): Q-1 (REQ-VAL-262): tier (a); Q-2 (REQ-VAL-264): tier (a); Q-3 (REQ-VAL-266): tier (a). *Was:* "The tier each binds is § '12.2 Tiers''s: (b); (a) statistically or qualitatively." (corrected 30 Oct 2026 to the owner's confirmation of the tier reading, R-649). Its calibrations are confirmed by the human with the M9 report (REQ-VAL-340), *was:* "at the M3 gate".
