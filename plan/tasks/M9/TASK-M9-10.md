# TASK-M9-10 — Agekyan & Anosova II: decay times from exact rest (AA-6, the first batch's DRAFT-VAL-05)

- **Milestone:** M9
- **Closes:** REQ-VAL-209, REQ-VAL-210
- **Depends on:** TASK-M3-34, TASK-M2-20
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-2, PIT-3
- **Size:** ~300 lines

## Goal
The replication suite's targets AA-6 run as engine tests (`principia_dd_replication.md`; R-594, R-629). For the 13 exact-rest region-D starts of Agekyan & Anosova II (Table 1) with T ≤ 10τ (N = 2, 7, 18, 43, 50, 51, 59, 61, 74, 85, 87, 95, 100; N = 50 a conditional decay), each run's decay time, converted to that row's own τ (paper I eq. 7 with L₀ = 0), must match the printed T within REQ-VAL-210's tolerance, under both the paper's decay criterion (Tevzadze, paper I eq. 10, transcribed) and ours, both recorded. Each tolerance is a calibration, proposed here with its evidence and confirmed by the human with the M9 report (R-71).

## References
- `docs/design/principia_dd_replication.md` § "5.8 Region D: Agekyan & Anosova, and Anosova 1969"
- `docs/design/principia_dd_replication.md` § "6. The first batch"
- `docs/design/principia_dd_replication.md` § "8.1 Carried over from the 25 Oct instruction"
- `docs/design/principia_dd_replication.md` § "8.2 Learnt since"
- `docs/design/principia_dd_replication.md` § "8.3 How the tasks apply them"
- `decisions.md` § "R-594 — The replication suite is adopted: published three-body results reproduced as engine tests in tiers A–D, ported as a design doc that extends the validation-orbits doc; each target a VAL requirement with its tolerance a calibration; every published table transcribed twice and never tuned to; sequenced by need; the questions it lists queued *(changes no ruling or requirement in force; recorded with R-629's port)*"
- `decisions.md` § "R-629 — PLAN.md is adopted as the single plan for verification and replication, superseding as the working plan the 25 Oct instruction's tier tables, sequencing and fetch list; its seven draft requirements are ported as M3 requirements with their tolerances as calibrations; R-594's port follows it, with the orbit catalogue's six tests as tasks; the plan's other questions stay queued *(refines R-594; answers PLAN.md § 9 questions 1 and 2)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/design/principia_dd_validation_orbits.md` § "1.6 Agekyan–Anosova region D — the near-rest starts (R-592)"
- `docs/notes/principia_validation_ground_truth_note.md` § "Four tiers of ground truth (increasing in what they prove)"
- `docs/notes/principia_validation_ground_truth_note.md` § "Chaos forbids pointwise comparison (exact vs structural)"

## Deliverables
- `fixtures/anosova/ii_table1_exact_rest.csv`, copied from the record's twice-transcribed CSV with its sha256, the record's diff cited.
- The remap (G = 6.67e-8 against 1; τ per row by paper I eq. 7) and Tevzadze's decay criterion (paper I eq. 10), each a tested function.
- `crates/validation/src/gates/anosova_decay.rs` and the calibration proposal.

## Acceptance tests
- `cargo xtask gate replication --target AA-6` — reference data docs/incoming/2026-10-27_papers/agekyan_anosova_II_table1_exact_rest.csv (twice-transcribed, its README § 6); the remap (G = 6.67e-8 in their units against G = 1; τ per row) is a tested function (rule 6); the energy column is recorded and not asserted (RQ-570's kind of discrepancy); the other 87 rows are compared as a distribution, not per row; control: a run with the remap's τ deliberately set from L₀ = 7.5e-9 (paper I's near-rest value) must move the comparison; each part's outcome recorded with its own tier, by § 12.2 ((b) on the 13 short-lived rows; (a) on the other 87): PASS where that part's comparison above holds, otherwise EXPLAINED DIVERGENCE (with the measurement showing the mechanism) or UNEXPLAINED, never a merge condition (R-649), a part not yet scored reading "not scored" with what it waits on; what scores the (a) part in this task, which has no GPU run, is RQ-669; the control still has to fire (REQ-VAL-209).
- `cargo xtask gate replication --target AA-6 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human with the M9 report and recorded in `decisions.md` (REQ-VAL-210).
- `git diff --no-index` of each fixture table's two transcriptions — empty, or each difference settled against the page and recorded beside the fixture, before the fixture is committed (R-594 item 4).

## Notes
- Calibrations proposed here (R-71; human confirmation with the M9 report, then recorded in decisions.md): REQ-VAL-210.
- R-594 item 4: every published table this task commits as a fixture is transcribed twice, independently, and the two copies are diffed before the commit, each row recording its page; both copies and the diff stay beside the fixture. A table a record has already transcribed twice and diffed is copied with its sha256 and the record's diff cited. Never tune to match: a disagreement with the paper goes to `REVIEW_QUEUE.md` with both numbers, as RQ-570 did, and the fixture is not edited.
- Part of the first batch (`principia_dd_replication.md` § 6), ported from PLAN.md § 6's drafts by R-629.
- Reference data are read from the records named in each requirement's verify; the records stay records (R-159), and the contract is `principia_dd_replication.md`.
- **R-632 (27 Oct 2026, RQ-570 (a)):** the energy column is recorded and never asserted; REQ-VAL-209 no longer carries `rq: RQ-570`.
- *30 Oct 2026 (R-649, R-650):* moved to M9, external replication, and renumbered: formerly TASK-M3-50 (milestone M3). Its targets are measurements, never gates: each is scored in M9, or early as a peek (information only, `docs/verification/peek_log.md`), and its outcome is PASS, EXPLAINED DIVERGENCE (with the measurement showing the mechanism) or UNEXPLAINED, never a failure that blocks; like-for-like is checked first, and nothing is tuned, loosened or special-cased to pass (`docs/design/principia_dd_replication.md` § "12.3 Outcomes, the like-for-like check and the forbidden moves"). The tier each target binds (§ "12.2 Tiers"; a calibration takes its target's): AA-6 (REQ-VAL-209): tier (b) on the 13 short-lived rows, tier (a) otherwise. *Was:* "The tier each binds is § '12.2 Tiers''s: (b); its statistical part also (a)." (corrected 30 Oct 2026 to the owner's confirmation of the tier reading, R-649). Its calibrations are confirmed by the human with the M9 report (REQ-VAL-340), *was:* "at the M3 gate".
- *1 Nov 2026 (physics review 5730823192 of PR #375, M2; applied per R-369):* AA-6 is a mixed target, scored on two tiers (`principia_dd_replication.md` § 12.2), so each part records its own outcome and TASK-M9-37 reads them part by part; a GPU miss on the (b) part is expected, not a defect, and a single outcome would hide which instrument agreed. *Was (acceptance):* "the outcome recorded with its tier: PASS where the comparison above holds, otherwise EXPLAINED DIVERGENCE …".
