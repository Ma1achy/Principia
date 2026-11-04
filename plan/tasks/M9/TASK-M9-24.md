# TASK-M9-24 — Parischewsky et al.'s equilateral-triangle experiment around Lagrange (PA-1)

- **Milestone:** M9
- **Closes:** REQ-VAL-323, REQ-VAL-324, REQ-VAL-343
- **Depends on:** TASK-M3-34
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-2, PIT-3
- **Size:** ~250 lines

## Goal
The replication suite's targets PA-1 run as engine tests (`principia_dd_replication.md`; R-594, R-629). Escaper-identity maps in side-ratio space around the Lagrange point must reproduce Parischewsky, Trani & Leigh 2023's, and their split, 82.1 % chaotic against 17.9 % regular (N_scramble ≥ 4), must be compared as a distribution (PA-1). Each tolerance is a calibration, proposed here with its evidence and confirmed by the human with the M9 report (R-71).

## References
- `docs/design/principia_dd_replication.md` § "5.12 Targets that wait for a chart, a diagnostic or data"
- `docs/design/principia_dd_replication.md` § "8.1 Carried over from the 25 Oct instruction"
- `docs/design/principia_dd_replication.md` § "8.2 Learnt since"
- `docs/design/principia_dd_replication.md` § "8.3 How the tasks apply them"
- `decisions.md` § "R-594 — The replication suite is adopted: published three-body results reproduced as engine tests in tiers A–D, ported as a design doc that extends the validation-orbits doc; each target a VAL requirement with its tolerance a calibration; every published table transcribed twice and never tuned to; sequenced by need; the questions it lists queued *(changes no ruling or requirement in force; recorded with R-629's port)*"
- `decisions.md` § "R-629 — PLAN.md is adopted as the single plan for verification and replication, superseding as the working plan the 25 Oct instruction's tier tables, sequencing and fetch list; its seven draft requirements are ported as M3 requirements with their tolerances as calibrations; R-594's port follows it, with the orbit catalogue's six tests as tasks; the plan's other questions stay queued *(refines R-594; answers PLAN.md § 9 questions 1 and 2)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-669 — The owner adopts the recommendations on the fifteen questions of 1 Nov but RQ-656: the Šuvakov-plane chart is one M9 task, its five targets waiting on it whole; a syzygy-symbol field only; N_scramble only; no periodic-orbit continuation, after the SJTU partial check; TR-1, MK-1 and SL-1 stay in M9, not scored, with no chart; the census's CI cadence as § 8.1 and § 8.2 write it, lavapipe in the smoke job; REQ-VAL-199 keeps 256 starts; GG-1's panels by like ε; the drift predicates read the stored latch, live values only for control flow; BZ-1C's starts by Path B, a list-of-starts chart the fallback; Hénon 1974 joins the rotating-frame set; Heggie & Sweatman 1991 in M9, not scored *(closes RQ-593, RQ-595, RQ-596, RQ-597, RQ-608, RQ-617, RQ-618, RQ-620, RQ-636, RQ-647 and RQ-654; amends R-636, R-637 and R-639; refines R-642)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/notes/principia_validation_ground_truth_note.md` § "Four tiers of ground truth (increasing in what they prove)"
- `docs/notes/principia_validation_ground_truth_note.md` § "Chaos forbids pointwise comparison (exact vs structural)"

## Deliverables
- The work on the chart RQ-594 rules; N_scramble, defined first (R-72; REQ-VAL-343) beside PA-1's row of `principia_dd_replication.md` § 5.12, both papers' words quoted, approved by the physics reviewer, then built (R-669, RQ-596 (b)). *Was:* "The work on the chart RQ-594 rules and the N_scramble diagnostic RQ-596 rules."
- `crates/validation/src/gates/side_ratio.rs`; the calibration proposal.

## Acceptance tests
- `cargo xtask gate replication --target PA-1` — the chart RQ-594 rules, the split once RQ-596 is; the maps reviewed by the physics reviewer; the split at REQ-VAL-324's level; control: a map at the wrong side ratio range must misplace the regular region; the outcome recorded with its tier: PASS where the comparison above holds, otherwise EXPLAINED DIVERGENCE (with the measurement showing the mechanism) or UNEXPLAINED, never a merge condition (R-649); the control still has to fire (REQ-VAL-323).
- `cargo xtask gate replication --target PA-1 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human with the M9 report and recorded in `decisions.md` (REQ-VAL-324).
- Review checklist (physics reviewer) — N_scramble's definition written beside PA-1's row of `principia_dd_replication.md` § 5.12, Parischewsky, Trani & Leigh 2023's and Stone & Leigh 2019's words quoted, approved; where the two papers differ, the difference stated and queued, never chosen (REQ-VAL-343).

## Notes
- Calibrations proposed here (R-71; human confirmation with the M9 report, then recorded in decisions.md): REQ-VAL-324.
- Waits on RQ-594: the task starts only once it is ruled (RQ-596 is ruled by R-669; *was:* "Waits on RQ-594, RQ-596: the task starts only once they are ruled"), and builds no chart or diagnostic ahead of the ruling (R-594 item 5).
- Reference data are read from the records named in each requirement's verify; the records stay records (R-159), and the contract is `principia_dd_replication.md`.
- *30 Oct 2026 (R-649, R-650):* moved to M9, external replication, and renumbered: formerly TASK-M3-64 (milestone M3). Its targets are measurements, never gates: each is scored in M9, or early as a peek (information only, `docs/verification/peek_log.md`), and its outcome is PASS, EXPLAINED DIVERGENCE (with the measurement showing the mechanism) or UNEXPLAINED, never a failure that blocks; like-for-like is checked first, and nothing is tuned, loosened or special-cased to pass (`docs/design/principia_dd_replication.md` § "12.3 Outcomes, the like-for-like check and the forbidden moves"). The tier each target binds (§ "12.2 Tiers"; a calibration takes its target's): PA-1 (REQ-VAL-323): tier (a). *Was:* "The tier each binds is § '12.2 Tiers''s: (b); (a) statistically or qualitatively." (corrected 30 Oct 2026 to the owner's confirmation of the tier reading, R-649). Its calibrations are confirmed by the human with the M9 report (REQ-VAL-340), *was:* "at the M3 gate". A wait on a ruling or data below holds this task's scoring, never a milestone.
- *1 Nov 2026 (R-669, RQ-596 (b)):* N_scramble only, defined and built here, PA-1 being the target that runs on a chart; SL-1 (TASK-M9-23) reads it once a binary-plus-single chart exists. f_col is not built. "the split once RQ-596 is" in the acceptance reads as REQ-VAL-343.
- *2 Nov 2026 (R-671, closing RQ-669):* R-671 reads the f64 runs of a task with no GPU run, and this task has no runs while RQ-594 is open, so PA-1 reads "not scored" with RQ-594 as what it waits on, with no record at (b). Once it has run with no GPU run, each of its (a) targets keeps its tier and reads "not scored yet: needs a GPU run" in M9's report, and the statistic its acceptance names, computed from the task's f64 runs, is recorded beside it at (b), its outcome and instrument stated. The tier is not relabelled; BZ-1C measures whether f32's statistics agree.
